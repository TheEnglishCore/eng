//! V1 package manager for Engling.
//!
//! This module implements the official/registry and community/URL package
//! flows described in `docs/PACKAGES.md`. It deliberately stays small:
//!
//! * No full dependency solver — packages can declare dependencies but
//!   resolution is "install if missing, same version".
//! * No sandbox — extracted files are inspected but not executed during
//!   install.
//! * No network at compile-time. HTTP is provided through an abstraction
//!   so tests can drive the manager with a local file-based fetcher.

pub mod commands;
pub mod fetcher;
pub mod installer;
pub mod manifest;
pub mod registry;
pub mod source;
pub mod store;
pub mod version;

pub use commands::{
    detect_dependency_cycle, format_list, format_search, install_source, list_installed,
    remove_package, search_registry, update_installed,
};
pub use fetcher::{Fetcher, HttpFetcher, LocalFetcher, ReadSeek};
pub use installer::{extract_zip, rebuild_index, sanitize_entry_path, verify_checksum, Installer};
pub use manifest::{validate_package_name, PackageManifest};
pub use registry::{Registry, RegistryEntry};
pub use source::{
    detect_source_for, fetch_github_zip_url, is_github_repo_url, parse_github_repo, PackageSource,
    SourceKind,
};
pub use store::{InstalledPackage, PackageIndex, PackageStore};
pub use version::{compare_versions, parse_version, Version};

/// Default official registry URL. Used when `ENGLING_REGISTRY` is unset.
pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/TheEnglishCore/eng-packages/main/registry.json";

/// Environment variable name that overrides the registry URL.
pub const REGISTRY_ENV_VAR: &str = "ENGLING_REGISTRY";

/// Resolve which registry URL to use, honoring `ENGLING_REGISTRY`.
pub fn resolve_registry_url() -> String {
    std::env::var(REGISTRY_ENV_VAR)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_string())
}
