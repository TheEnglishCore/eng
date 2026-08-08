//! On-disk package store.
//!
//! Layout:
//!
//! ```text
//! <root>/
//!     manifest_index.json        # map: name -> {version, installed_at}
//!     <name>/
//!         manifest.json
//!         README.md
//!         src/...
//! ```
//!
//! The root defaults to `~/.engling/packages` on Unix-like systems
//! (including Termux) and `%USERPROFILE%/.engling/packages` on
//! Windows. Callers can override the root via the `ENGLING_PACKAGES_DIR`
//! environment variable or by constructing a [`PackageStore`] directly.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::EnglingError;
use crate::package::manifest::PackageManifest;
use crate::package::version::Version;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub installed_at_unix: u64,
    /// Original source so we know whether `eng update` is safe.
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageIndex(BTreeMap<String, InstalledPackage>);

impl PackageIndex {
    fn path(root: &Path) -> PathBuf {
        root.join("manifest_index.json")
    }

    pub fn load(root: &Path) -> Self {
        let path = Self::path(root);
        if let Ok(text) = fs::read_to_string(&path) {
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, root: &Path) -> Result<(), EnglingError> {
        let path = Self::path(root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                EnglingError::package(format!(
                    "could not create store directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| EnglingError::package(format!("could not serialize index: {e}")))?;
        fs::write(&path, text).map_err(|e| {
            EnglingError::package(format!("could not write {}: {e}", path.display()))
        })?;
        Ok(())
    }

    pub fn entries(&self) -> impl Iterator<Item = &InstalledPackage> {
        self.0.values()
    }

    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.0.get(name)
    }

    pub fn insert(&mut self, pkg: InstalledPackage) {
        self.0.insert(pkg.name.clone(), pkg);
    }

    pub fn remove(&mut self, name: &str) -> Option<InstalledPackage> {
        self.0.remove(name)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
pub struct PackageStore {
    root: PathBuf,
}

impl PackageStore {
    /// Create a store rooted at the user default location. Honors
    /// `ENGLING_PACKAGES_DIR` if set.
    pub fn user_default() -> Self {
        let root = user_packages_dir();
        Self { root }
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_root(&self) -> Result<(), EnglingError> {
        fs::create_dir_all(&self.root).map_err(|e| {
            EnglingError::package(format!(
                "could not create package directory {}: {e}",
                self.root.display()
            ))
        })
    }

    pub fn package_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn manifest_path(&self, name: &str) -> PathBuf {
        self.package_dir(name).join("manifest.json")
    }

    /// Read the installed manifest, if any.
    pub fn load_manifest(&self, name: &str) -> Result<Option<PackageManifest>, EnglingError> {
        let path = self.manifest_path(name);
        if !path.exists() {
            return Ok(None);
        }
        PackageManifest::from_file(&path).map(Some)
    }

    pub fn index(&self) -> PackageIndex {
        PackageIndex::load(&self.root)
    }

    pub fn save_index(&self, idx: &PackageIndex) -> Result<(), EnglingError> {
        idx.save(&self.root)
    }

    pub fn is_installed(&self, name: &str) -> bool {
        self.manifest_path(name).exists()
    }

    pub fn remove(&self, name: &str) -> Result<(), EnglingError> {
        if !self.is_installed(name) {
            return Err(EnglingError::package(format!(
                "package '{name}' is not installed"
            )));
        }
        let dir = self.package_dir(name);
        fs::remove_dir_all(&dir).map_err(|e| {
            EnglingError::package(format!("could not remove {}: {e}", dir.display()))
        })?;
        // Best-effort cleanup of stale index entries.
        let mut idx = self.index();
        idx.remove(name);
        self.save_index(&idx)?;
        Ok(())
    }

    /// Return the absolute path to the `.eng` file that `import X.`
    /// should resolve to, given an installed package `X`. Returns
    /// `None` if no installed manifest matches.
    pub fn resolve_module(&self, name: &str) -> Option<PathBuf> {
        let manifest = self.load_manifest(name).ok().flatten()?;
        Some(self.package_dir(name).join(manifest.main))
    }
}

/// Determine the default user packages directory.
pub fn user_packages_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("ENGLING_PACKAGES_DIR") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".engling").join("packages");
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(profile).join(".engling").join("packages");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".engling").join("packages");
    }
    PathBuf::from(".engling/packages")
}

#[allow(dead_code)]
fn timestamp_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Re-exported for convenience.
pub use InstalledPackage as Record;

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, PackageStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();
        (dir, store)
    }

    #[test]
    fn index_round_trip() {
        let (_dir, store) = temp_store();
        let mut idx = store.index();
        idx.insert(InstalledPackage {
            name: "colors".to_string(),
            version: "1.0.0".to_string(),
            installed_at_unix: 1,
            source: "colors".to_string(),
        });
        store.save_index(&idx).unwrap();
        let loaded = store.index();
        assert_eq!(loaded.get("colors").unwrap().version, "1.0.0");
    }

    #[test]
    fn remove_missing_errors() {
        let (_dir, store) = temp_store();
        let err = store.remove("nope").unwrap_err();
        assert!(format!("{err}").contains("not installed"));
    }

    #[test]
    fn user_default_uses_env_override() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("ENGLING_PACKAGES_DIR", tmp.path());
        let p = user_packages_dir();
        std::env::remove_var("ENGLING_PACKAGES_DIR");
        assert_eq!(p, tmp.path());
    }

    #[test]
    fn resolve_module_uses_manifest_main() {
        let (_dir, store) = temp_store();
        let dir = store.package_dir("colors");
        fs::create_dir_all(dir.join("src")).unwrap();
        let main = dir.join("src").join("colors.eng");
        fs::write(&main, "Print 1.").unwrap();
        let manifest = PackageManifest {
            name: "colors".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            main: "src/colors.eng".to_string(),
            license: "".to_string(),
            dependencies: vec![],
            checksum: None,
            update_url: None,
        };
        manifest.write_to(&store.manifest_path("colors")).unwrap();
        let resolved = store.resolve_module("colors").unwrap();
        assert_eq!(resolved, main);
    }
}

// Suppress unused-import on Version (re-exported for users).
#[allow(dead_code)]
fn _force_version_import(v: Version) -> Version {
    v
}
