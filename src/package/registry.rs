//! Official registry loading.
//!
//! The registry is a JSON object keyed by package name. Each entry
//! describes a release. The shape is intentionally simple so a
//! community member can hand-edit one if they ever need to:
//!
//! ```json
//! {
//!     "colors": {
//!         "version": "1.0.0",
//!         "description": "Color utilities for Engling",
//!         "url": "https://github.com/.../colors-1.0.0.engpkg",
//!         "checksum": "<sha256 hex>"
//!     }
//! }
//! ```

use std::collections::BTreeMap;
use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::error::EnglingError;
use crate::package::fetcher::Fetcher;
use crate::package::version::parse_version;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryEntry {
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub url: String,
    #[serde(default)]
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Registry {
    pub packages: BTreeMap<String, RegistryEntry>,
}

impl Registry {
    /// Parse a registry from a JSON string.
    pub fn from_json(text: &str) -> Result<Self, EnglingError> {
        let value: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| EnglingError::package(format!("registry is invalid JSON: {e}")))?;
        let obj = value.as_object().ok_or_else(|| {
            EnglingError::package("registry must be a JSON object keyed by package name")
        })?;
        let mut packages = BTreeMap::new();
        for (name, raw) in obj {
            crate::package::manifest::validate_package_name(name)?;
            let entry: RegistryEntry = serde_json::from_value(raw.clone()).map_err(|e| {
                EnglingError::package(format!("registry entry '{name}' is invalid: {e}"))
            })?;
            parse_version(&entry.version)?;
            // Registry URL must be http(s).
            let lower = entry.url.to_lowercase();
            if !(lower.starts_with("http://") || lower.starts_with("https://")) {
                return Err(EnglingError::package(format!(
                    "registry entry '{name}' has invalid URL '{}'",
                    entry.url
                )));
            }
            // Normalize: an empty checksum string means "no checksum".
            let mut entry = entry;
            if let Some(c) = entry.checksum.as_deref() {
                if c.trim().is_empty() {
                    entry.checksum = None;
                }
            }
            packages.insert(name.clone(), entry);
        }
        Ok(Registry { packages })
    }

    /// Load a registry from an HTTP(S) URL.
    pub fn load_url(url: &str, fetcher: &dyn Fetcher) -> Result<Self, EnglingError> {
        let mut reader = fetcher.fetch(url)?;
        let mut text = String::new();
        reader
            .read_to_string(&mut text)
            .map_err(|e| EnglingError::package(format!("could not read registry '{url}': {e}")))?;
        Self::from_json(&text)
    }

    pub fn lookup(&self, name: &str) -> Option<&RegistryEntry> {
        self.packages.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.packages.keys().map(|s| s.as_str())
    }

    pub fn search(&self, query: &str) -> Vec<(&str, &RegistryEntry)> {
        let q = query.to_lowercase();
        self.packages
            .iter()
            .filter(|(name, entry)| {
                name.to_lowercase().contains(&q) || entry.description.to_lowercase().contains(&q)
            })
            .map(|(name, entry)| (name.as_str(), entry))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry() {
        let json = r#"{
            "colors": {
                "version": "1.0.0",
                "description": "Color utilities",
                "url": "https://example.com/colors.engpkg",
                "checksum": "abc123"
            },
            "math": {
                "version": "0.2.0",
                "description": "Math helpers",
                "url": "https://example.com/math.engpkg"
            }
        }"#;
        let reg = Registry::from_json(json).unwrap();
        assert_eq!(reg.packages.len(), 2);
        let colors = reg.lookup("colors").unwrap();
        assert_eq!(colors.version, "1.0.0");
        assert_eq!(colors.checksum.as_deref(), Some("abc123"));
        assert!(reg.lookup("nope").is_none());
    }

    #[test]
    fn rejects_invalid_version() {
        let json = r#"{
            "colors": {
                "version": "1.0",
                "url": "https://example.com/colors.engpkg"
            }
        }"#;
        let err = Registry::from_json(json).unwrap_err();
        assert!(format!("{err}").contains("major.minor.patch"));
    }

    #[test]
    fn rejects_non_url() {
        let json = r#"{
            "colors": {
                "version": "1.0.0",
                "url": "ftp://example.com/colors.engpkg"
            }
        }"#;
        let err = Registry::from_json(json).unwrap_err();
        assert!(format!("{err}").contains("invalid URL"));
    }

    #[test]
    fn rejects_invalid_name() {
        let json = r#"{
            "9bad": {
                "version": "1.0.0",
                "url": "https://example.com/x.engpkg"
            }
        }"#;
        let err = Registry::from_json(json).unwrap_err();
        assert!(format!("{err}").contains("package name"));
    }

    #[test]
    fn search_finds_matches() {
        let json = r#"{
            "colors": {"version":"1.0.0","description":"Color helpers","url":"https://e.test/c.engpkg"},
            "math": {"version":"1.0.0","description":"Math helpers","url":"https://e.test/m.engpkg"}
        }"#;
        let reg = Registry::from_json(json).unwrap();
        let hits = reg.search("color");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "colors");
        let hits = reg.search("helpers");
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn rejects_non_object_root() {
        let err = Registry::from_json("[1, 2, 3]").unwrap_err();
        assert!(format!("{err}").contains("JSON object"));
    }
}
