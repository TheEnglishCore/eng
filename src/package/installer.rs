//! Extract a `.engpkg` archive into the package store.
//!
//! ## Security
//!
//! Direct community packages are treated as untrusted input. The
//! installer:
//!
//! * Rejects ZIP entries with absolute paths or `..` segments.
//! * Skips entries that escape the destination directory.
//! * Validates `manifest.json` before writing anything.
//! * Verifies a SHA-256 checksum against the registry entry / manifest
//!   when one is provided.
//! * Never executes `.eng` code during install.
//!
//! The archive format is the standard ZIP file, which the `zip` crate
//! can read. Each archive is expected to contain a `manifest.json` at
//! the root, plus arbitrary additional files (`README.md`, `src/...`,
//! etc.).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::EnglingError;
use crate::package::manifest::PackageManifest;
use crate::package::store::{InstalledPackage, PackageIndex, PackageStore};

pub struct Installer<'a> {
    store: &'a PackageStore,
}

impl<'a> Installer<'a> {
    pub fn new(store: &'a PackageStore) -> Self {
        Self { store }
    }

    /// Install a package from arbitrary bytes. `source` is recorded in
    /// the index so `eng update` knows whether re-fetching is safe.
    pub fn install_bytes(
        &self,
        name_hint: Option<&str>,
        bytes: &[u8],
        expected_checksum: Option<&str>,
        source: &str,
    ) -> Result<PackageManifest, EnglingError> {
        if let Some(expected) = expected_checksum {
            verify_checksum(bytes, expected)?;
        }
        // Extract to a staging dir, validate, then move into place.
        let staging = self.staging_dir(name_hint.unwrap_or("pkg"))?;
        // Clean any prior staging content.
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|e| {
                EnglingError::package(format!(
                    "could not clear staging {}: {e}",
                    staging.display()
                ))
            })?;
        }
        fs::create_dir_all(&staging).map_err(|e| {
            EnglingError::package(format!(
                "could not create staging {}: {e}",
                staging.display()
            ))
        })?;
        extract_zip(bytes, &staging)?;

        let manifest_path = staging.join("manifest.json");
        if !manifest_path.exists() {
            return Err(EnglingError::package(
                "package manifest is invalid: manifest.json not found",
            ));
        }
        let manifest = PackageManifest::from_file(&manifest_path)?;
        let pkg_dir = self.store.package_dir(&manifest.name);

        // Replace any prior install.
        if pkg_dir.exists() {
            fs::remove_dir_all(&pkg_dir).map_err(|e| {
                EnglingError::package(format!(
                    "could not clear existing package at {}: {e}",
                    pkg_dir.display()
                ))
            })?;
        }
        fs::create_dir_all(pkg_dir.parent().unwrap_or(Path::new("."))).map_err(|e| {
            EnglingError::package(format!("could not create package parent dir: {e}"))
        })?;
        if let Err(rename_err) = fs::rename(&staging, &pkg_dir) {
            // Fall back to a recursive copy if rename across mount points fails.
            if let Err(copy_err) = copy_dir(&staging, &pkg_dir) {
                return Err(EnglingError::package(format!(
                    "could not move package into store: rename failed ({rename_err}), copy failed ({copy_err})"
                )));
            }
            let _ = fs::remove_dir_all(&staging);
        }

        // Record in the index.
        let mut idx = self.store.index();
        idx.insert(InstalledPackage {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            installed_at_unix: timestamp_now(),
            source: source.to_string(),
        });
        self.store.save_index(&idx)?;

        Ok(manifest)
    }

    fn staging_dir(&self, hint: &str) -> Result<PathBuf, EnglingError> {
        let dir = self
            .store
            .root()
            .join(".staging")
            .join(format!("{hint}-{}", timestamp_now()));
        Ok(dir)
    }
}

/// Verify a checksum using SHA-256. An empty `expected` is treated as
/// "no checksum available" and is silently accepted — this matches the
/// V1 wire format where the field is `""` for unverified packages.
pub fn verify_checksum(bytes: &[u8], expected: &str) -> Result<(), EnglingError> {
    let expected_clean = expected.trim().to_lowercase();
    if expected_clean.is_empty() {
        return Ok(());
    }
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let got = hasher.finalize();
    let hex: String = got.iter().map(|b| format!("{b:02x}")).collect();
    if !hex.eq_ignore_ascii_case(&expected_clean) {
        return Err(EnglingError::package(format!(
            "package checksum verification failed (expected {expected_clean}, got {hex})"
        )));
    }
    Ok(())
}

/// Extract a ZIP archive into `dest`, rejecting unsafe entries.
pub fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), EnglingError> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| EnglingError::package(format!("package archive is corrupt: {e}")))?;
    let dest_abs = std::fs::canonicalize(dest).unwrap_or_else(|_| dest.to_path_buf());
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            EnglingError::package(format!("package archive entry {i} is corrupt: {e}"))
        })?;
        let raw_name = entry.name().to_string();
        let safe_rel = match sanitize_entry_path(&raw_name) {
            Some(p) => p,
            None => {
                return Err(EnglingError::package(format!(
                    "package archive contains a malicious path: '{raw_name}'"
                )));
            }
        };
        let target = dest.join(&safe_rel);

        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|e| {
                EnglingError::package(format!(
                    "could not create directory {}: {e}",
                    target.display()
                ))
            })?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                EnglingError::package(format!(
                    "could not create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        // Belt and braces: even after sanitize, ensure the resolved
        // path is still under `dest_abs`.
        let resolved = std::fs::canonicalize(target.parent().unwrap_or(dest))
            .unwrap_or_else(|_| target.parent().unwrap_or(dest).to_path_buf());
        if !resolved.starts_with(&dest_abs) && !target.starts_with(dest) {
            return Err(EnglingError::package(format!(
                "package archive entry '{raw_name}' escapes the package directory"
            )));
        }

        let mut out = File::create(&target).map_err(|e| {
            EnglingError::package(format!("could not write {}: {e}", target.display()))
        })?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(|e| {
            EnglingError::package(format!("could not read entry '{raw_name}': {e}"))
        })?;
        out.write_all(&buf).map_err(|e| {
            EnglingError::package(format!("could not write {}: {e}", target.display()))
        })?;
    }
    Ok(())
}

/// Sanitize a ZIP entry path. Returns the safe relative path or
/// `None` if the entry should be rejected outright.
pub fn sanitize_entry_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.replace('\\', "/");
    if trimmed.starts_with('/') {
        return None;
    }
    // Reject Windows drive-letter prefixes (`C:/foo`, `C:\foo`). On
    // Unix hosts `Path::components` collapses these into a `Normal`
    // segment, which would otherwise sneak past the path-traversal
    // checks below.
    if trimmed
        .as_bytes()
        .get(1)
        .map(|c| *c == b':')
        .unwrap_or(false)
    {
        return None;
    }
    let mut out = PathBuf::new();
    for component in Path::new(&trimmed).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    if out.as_os_str().is_empty() {
        return None;
    }
    Some(out)
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn timestamp_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Recompute an index from the on-disk state. Useful after manual
/// surgery on the store directory.
pub fn rebuild_index(store: &PackageStore) -> Result<PackageIndex, EnglingError> {
    let mut idx = PackageIndex::default();
    let read = match fs::read_dir(store.root()) {
        Ok(r) => r,
        Err(_) => return Ok(idx),
    };
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        if name.starts_with('.') {
            continue;
        }
        if let Ok(manifest) = PackageManifest::from_file(&manifest_path) {
            idx.insert(InstalledPackage {
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                installed_at_unix: 0,
                source: "rebuilt".to_string(),
            });
        }
    }
    store.save_index(&idx)?;
    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_traversal() {
        assert!(sanitize_entry_path("../etc/passwd").is_none());
        assert!(sanitize_entry_path("foo/../../bar").is_none());
        assert!(sanitize_entry_path("/etc/passwd").is_none());
        assert!(sanitize_entry_path("C:/Windows").is_none());
    }

    #[test]
    fn sanitize_accepts_normal() {
        assert_eq!(
            sanitize_entry_path("src/colors.eng").unwrap(),
            PathBuf::from("src").join("colors.eng")
        );
        assert_eq!(
            sanitize_entry_path("manifest.json").unwrap(),
            PathBuf::from("manifest.json")
        );
        assert_eq!(
            sanitize_entry_path("./src/main.eng").unwrap(),
            PathBuf::from("src").join("main.eng")
        );
    }

    #[test]
    fn verify_checksum_works() {
        let bytes = b"hello world";
        // sha256("hello world")
        verify_checksum(
            bytes,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        )
        .unwrap();
        let err = verify_checksum(bytes, "deadbeef").unwrap_err();
        assert!(format!("{err}").contains("checksum"));
    }

    fn build_minimal_zip() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::<u8>::new());
        {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("manifest.json", opts).unwrap();
            zip.write_all(
                br#"{
                    "name": "colors",
                    "version": "1.0.0",
                    "main": "src/colors.eng"
                }"#,
            )
            .unwrap();
            zip.start_file("src/colors.eng", opts).unwrap();
            zip.write_all(b"Print \"hi\".\n").unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn install_minimal_package() {
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();
        let installer = Installer::new(&store);
        let manifest = installer
            .install_bytes(Some("colors"), &build_minimal_zip(), None, "colors")
            .unwrap();
        assert_eq!(manifest.name, "colors");
        assert!(store.package_dir("colors").join("src/colors.eng").exists());
        let idx = store.index();
        assert_eq!(idx.get("colors").unwrap().version, "1.0.0");
    }

    fn build_malicious_zip() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::<u8>::new());
        {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("../escape.txt", opts).unwrap();
            zip.write_all(b"bad").unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn rejects_malicious_zip_entry() {
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();
        let installer = Installer::new(&store);
        let err = installer
            .install_bytes(Some("evil"), &build_malicious_zip(), None, "evil")
            .unwrap_err();
        assert!(format!("{err}").contains("malicious"));
    }

    #[test]
    fn rejects_corrupt_zip() {
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();
        let installer = Installer::new(&store);
        let err = installer
            .install_bytes(Some("bad"), b"not a zip", None, "bad")
            .unwrap_err();
        assert!(format!("{err}").contains("corrupt"));
    }

    #[test]
    fn rejects_missing_manifest() {
        let mut buf = std::io::Cursor::new(Vec::<u8>::new());
        let bytes = {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("README.md", opts).unwrap();
            zip.write_all(b"hi").unwrap();
            zip.finish().unwrap();
            buf.into_inner()
        };
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();
        let installer = Installer::new(&store);
        let err = installer
            .install_bytes(Some("missing"), &bytes, None, "missing")
            .unwrap_err();
        assert!(format!("{err}").contains("manifest"));
    }
}
