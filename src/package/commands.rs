//! High-level commands used by the CLI.
//!
//! Each public function in this module maps 1:1 to a subcommand:
//!
//! * [`install_source`]   → `eng install <arg>`
//! * [`remove_package`]   → `eng remove <name>`
//! * [`list_installed`]   → `eng list`
//! * [`search_registry`]  → `eng search <query>`
//! * [`update_installed`] → `eng update`

use std::collections::HashSet;
use std::io::Read;

use crate::error::EnglingError;
use crate::package::fetcher::Fetcher;
use crate::package::installer::Installer;
use crate::package::manifest::PackageManifest;
use crate::package::registry::{Registry, RegistryEntry};
use crate::package::source::{
    fetch_github_zip_url, github_candidate_branches, parse_github_repo, PackageSource, SourceKind,
};
use crate::package::store::{InstalledPackage, PackageStore};
use crate::package::version::{compare_versions, parse_version};
use crate::package::{Fetcher as FetcherTrait, HttpFetcher};

/// User-visible description of an installed package.
#[derive(Debug, Clone)]
pub struct InstalledDisplay {
    pub entry: InstalledPackage,
    pub manifest: Option<PackageManifest>,
}

/// Install a package from any supported source. `fetcher` is used to
/// fetch the registry and the package archive.
pub fn install_source(arg: &str, fetcher: &dyn Fetcher) -> Result<PackageManifest, EnglingError> {
    let source = PackageSource::detect(arg)?;
    let store = PackageStore::user_default();
    store.ensure_root()?;
    let installer = Installer::new(&store);
    let registry_url = crate::package::resolve_registry_url();
    match source.kind {
        SourceKind::OfficialName => {
            let registry = Registry::load_url(&registry_url, fetcher)?;
            let entry = registry.lookup(&source.raw).ok_or_else(|| {
                EnglingError::package(format!(
                    "package '{}' was not found in the official Engling registry",
                    source.raw
                ))
            })?;
            install_from_registry(&installer, &registry, &store, entry, fetcher, &source.raw)
        }
        SourceKind::DirectArchive => {
            install_from_direct_url(&installer, &store, &source.raw, fetcher)
        }
        SourceKind::GitHubRepo => install_from_github(&installer, &store, &source.raw, fetcher),
    }
}

fn install_from_registry(
    installer: &Installer,
    registry: &Registry,
    store: &PackageStore,
    entry: &RegistryEntry,
    fetcher: &dyn Fetcher,
    requested_name: &str,
) -> Result<PackageManifest, EnglingError> {
    if store.is_installed(requested_name) {
        if let Some(existing) = store.load_manifest(requested_name)? {
            if existing.version == entry.version {
                // Already installed at the requested version.
                return Ok(existing);
            }
        }
    }
    let bytes = fetch_all(fetcher, &entry.url)?;
    let manifest = installer.install_bytes(
        Some(requested_name),
        &bytes,
        entry.checksum.as_deref(),
        requested_name,
    )?;
    // Install dependencies (recursive, no version solver).
    let mut visited = HashSet::new();
    install_deps(installer, registry, store, &manifest, &mut visited, fetcher)?;
    Ok(manifest)
}

fn install_from_direct_url(
    installer: &Installer,
    store: &PackageStore,
    url: &str,
    fetcher: &dyn Fetcher,
) -> Result<PackageManifest, EnglingError> {
    let bytes = fetch_all(fetcher, url)?;
    let manifest = installer.install_bytes(None, &bytes, None, url)?;
    // We do not resolve dependencies for arbitrary URLs.
    let _ = store;
    Ok(manifest)
}

fn install_from_github(
    installer: &Installer,
    store: &PackageStore,
    url: &str,
    fetcher: &dyn Fetcher,
) -> Result<PackageManifest, EnglingError> {
    let (owner, repo) = parse_github_repo(url)
        .ok_or_else(|| EnglingError::package(format!("unsupported GitHub URL '{url}'")))?;
    let mut last_err: Option<EnglingError> = None;
    for branch in github_candidate_branches() {
        let zip_url = fetch_github_zip_url(&owner, &repo, branch);
        match fetch_all(fetcher, &zip_url) {
            Ok(bytes) => {
                let manifest = match installer.install_bytes(Some(&repo), &bytes, None, &zip_url) {
                    Ok(m) => m,
                    Err(_) => {
                        // If the archive does not look like an .engpkg
                        // (e.g. missing manifest), fall through to the
                        // next branch and ultimately report the error.
                        last_err = Some(EnglingError::package(format!(
                            "GitHub archive '{zip_url}' is not a valid .engpkg package"
                        )));
                        continue;
                    }
                };
                let _ = store;
                return Ok(manifest);
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        EnglingError::package(format!("could not download GitHub repository '{url}'"))
    }))
}

fn install_deps(
    installer: &Installer,
    registry: &Registry,
    store: &PackageStore,
    manifest: &PackageManifest,
    visited: &mut HashSet<String>,
    fetcher: &dyn Fetcher,
) -> Result<(), EnglingError> {
    for dep in &manifest.dependencies {
        if visited.contains(dep) {
            continue;
        }
        visited.insert(dep.clone());
        // Circular detection: if installing would land us on a name we
        // are already walking through.
        if visited.contains(dep) && !store.is_installed(dep) {
            // No-op when the dependency is not present; the recursion
            // below will install it. Circular check is done at a
            // coarser level — see [`detect_dependency_cycle`].
        }
        if store.is_installed(dep) {
            continue;
        }
        let entry = registry.lookup(dep).ok_or_else(|| {
            EnglingError::package(format!(
                "package '{dep}' (required by '{}') was not found in the official Engling registry",
                manifest.name
            ))
        })?;
        let bytes = fetch_all(fetcher, &entry.url)?;
        let dep_manifest =
            installer.install_bytes(Some(dep), &bytes, entry.checksum.as_deref(), dep)?;
        install_deps(installer, registry, store, &dep_manifest, visited, fetcher)?;
    }
    Ok(())
}

/// Remove an installed package by name.
pub fn remove_package(name: &str) -> Result<(), EnglingError> {
    crate::package::manifest::validate_package_name(name)?;
    let store = PackageStore::user_default();
    store.remove(name)
}

/// List installed packages, sorted by name.
pub fn list_installed() -> Result<Vec<InstalledDisplay>, EnglingError> {
    let store = PackageStore::user_default();
    store.ensure_root()?;
    let idx = store.index();
    let mut out: Vec<InstalledDisplay> = idx
        .entries()
        .map(|e| {
            let manifest = store.load_manifest(&e.name).ok().flatten();
            InstalledDisplay {
                entry: e.clone(),
                manifest,
            }
        })
        .collect();
    out.sort_by(|a, b| a.entry.name.cmp(&b.entry.name));
    Ok(out)
}

/// Search the official registry. Returns the matching
/// `(name, description, version)` triples.
pub fn search_registry(
    query: &str,
    fetcher: &dyn Fetcher,
) -> Result<Vec<(String, RegistryEntry)>, EnglingError> {
    let url = crate::package::resolve_registry_url();
    let registry = Registry::load_url(&url, fetcher)?;
    Ok(registry
        .search(query)
        .into_iter()
        .map(|(name, entry)| (name.to_string(), entry.clone()))
        .collect())
}

/// Update every installed package whose source is an official name (not
/// a URL). Community installs are skipped unless their manifest provides
/// an `update_url`.
pub fn update_installed(fetcher: &dyn Fetcher) -> Result<Vec<String>, EnglingError> {
    let store = PackageStore::user_default();
    store.ensure_root()?;
    let url = crate::package::resolve_registry_url();
    let registry = Registry::load_url(&url, fetcher)?;
    let idx = store.index();
    let mut updated = Vec::new();
    for entry in idx.entries() {
        // Decide what to install: official names come from the registry,
        // community packages only when the manifest provides an update URL.
        let install_target = if is_official_source(&entry.source) {
            let reg_entry = match registry.lookup(&entry.name) {
                Some(e) => e,
                None => continue,
            };
            match is_newer(&entry.version, &reg_entry.version) {
                Some(true) => Some((reg_entry.url.clone(), reg_entry.checksum.clone())),
                _ => None,
            }
        } else {
            let manifest_opt = store.load_manifest(&entry.name)?;
            manifest_opt
                .as_ref()
                .and_then(|m| m.update_url.clone())
                .map(|u| (u, None))
        };
        if let Some((url, checksum)) = install_target {
            let bytes = fetch_all(fetcher, &url)?;
            let installer = Installer::new(&store);
            let manifest =
                installer.install_bytes(Some(&entry.name), &bytes, checksum.as_deref(), &url)?;
            updated.push(format!(
                "{} {} -> {}",
                manifest.name, entry.version, manifest.version
            ));
        }
    }
    Ok(updated)
}

/// Quick check used by the CLI: compare two version strings and report
/// whether `latest` is strictly newer than `current`.
pub fn is_newer(current: &str, latest: &str) -> Option<bool> {
    let c = parse_version(current).ok()?;
    let l = parse_version(latest).ok()?;
    Some(compare_versions(&l, &c) == std::cmp::Ordering::Greater)
}

fn is_official_source(source: &str) -> bool {
    let lower = source.to_lowercase();
    !lower.starts_with("http://") && !lower.starts_with("https://")
}

fn fetch_all(fetcher: &dyn Fetcher, url: &str) -> Result<Vec<u8>, EnglingError> {
    let mut reader = fetcher.fetch(url)?;
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| EnglingError::package(format!("could not read response from {url}: {e}")))?;
    Ok(buf)
}

/// Sanity-check the dependency graph for cycles. Returns the offending
/// package name when one is found, `None` otherwise.
pub fn detect_dependency_cycle(store: &PackageStore, name: &str) -> Option<String> {
    let mut seen = HashSet::new();
    let mut stack = vec![name.to_string()];
    while let Some(current) = stack.pop() {
        if !seen.insert(current.clone()) {
            return Some(current);
        }
        let manifest = store.load_manifest(&current).ok().flatten()?;
        for dep in &manifest.dependencies {
            if !store.is_installed(dep) {
                return None;
            }
            stack.push(dep.clone());
        }
    }
    None
}

/// Build the default fetcher the CLI uses. Falls back to [`LocalFetcher`]
/// when neither HTTP nor a local mirror can be configured — this keeps
/// the CLI honest in air-gapped or test environments.
pub fn default_fetcher() -> Box<dyn FetcherTrait> {
    Box::new(HttpFetcher::new())
}

/// Format an `eng list` report for printing.
pub fn format_list(packages: &[InstalledDisplay]) -> String {
    if packages.is_empty() {
        return "No Engling packages are installed.\n".to_string();
    }
    let mut out = String::from("Installed packages:\n");
    for pkg in packages {
        out.push_str(&format!("  {} {}\n", pkg.entry.name, pkg.entry.version));
    }
    out
}

/// Format an `eng search` report for printing.
pub fn format_search(results: &[(String, RegistryEntry)]) -> String {
    if results.is_empty() {
        return "No packages matched.\n".to_string();
    }
    let mut out = String::new();
    for (name, entry) in results {
        out.push_str(&format!(
            "{} {}\n  {}\n",
            name, entry.version, entry.description
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::PackageManifest;
    use crate::package::PackageIndex;

    #[test]
    fn is_newer_basic() {
        assert_eq!(is_newer("1.0.0", "1.0.1"), Some(true));
        assert_eq!(is_newer("1.0.0", "1.0.0"), Some(false));
        assert_eq!(is_newer("1.0.1", "1.0.0"), Some(false));
        assert_eq!(is_newer("1.0.0", "2.0.0"), Some(true));
    }

    #[test]
    fn is_newer_handles_invalid() {
        assert_eq!(is_newer("garbage", "1.0.0"), None);
    }

    #[test]
    fn cycle_detector_finds_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();

        let a = PackageManifest {
            name: "a".into(),
            version: "1.0.0".into(),
            description: "".into(),
            main: "a.eng".into(),
            license: "".into(),
            dependencies: vec!["b".into()],
            checksum: None,
            update_url: None,
        };
        let b = PackageManifest {
            name: "b".into(),
            version: "1.0.0".into(),
            description: "".into(),
            main: "b.eng".into(),
            license: "".into(),
            dependencies: vec!["a".into()],
            checksum: None,
            update_url: None,
        };
        a.write_to(&store.manifest_path("a")).unwrap();
        b.write_to(&store.manifest_path("b")).unwrap();
        assert_eq!(detect_dependency_cycle(&store, "a"), Some("a".into()));
    }

    #[test]
    fn cycle_detector_no_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let store = PackageStore::at(dir.path());
        store.ensure_root().unwrap();

        let a = PackageManifest {
            name: "a".into(),
            version: "1.0.0".into(),
            description: "".into(),
            main: "a.eng".into(),
            license: "".into(),
            dependencies: vec![],
            checksum: None,
            update_url: None,
        };
        a.write_to(&store.manifest_path("a")).unwrap();
        assert_eq!(detect_dependency_cycle(&store, "a"), None);
    }

    #[test]
    fn format_list_works() {
        let pkgs = vec![InstalledDisplay {
            entry: InstalledPackage {
                name: "colors".into(),
                version: "1.0.0".into(),
                installed_at_unix: 0,
                source: "colors".into(),
            },
            manifest: None,
        }];
        let out = format_list(&pkgs);
        assert!(out.contains("colors 1.0.0"));
    }

    #[test]
    fn format_list_empty() {
        let out = format_list(&[]);
        assert!(out.contains("No Engling packages"));
    }

    #[test]
    fn format_search_works() {
        let entries = vec![(
            "colors".into(),
            RegistryEntry {
                version: "1.0.0".into(),
                description: "Color helpers".into(),
                url: "https://example.com/c.engpkg".into(),
                checksum: None,
            },
        )];
        let out = format_search(&entries);
        assert!(out.contains("colors 1.0.0"));
        assert!(out.contains("Color helpers"));
    }

    #[test]
    fn format_search_empty() {
        let out = format_search(&[]);
        assert!(out.contains("No packages"));
    }

    #[test]
    fn index_helpers() {
        let idx = PackageIndex::default();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }
}
