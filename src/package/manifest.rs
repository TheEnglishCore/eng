//! Package manifest (`manifest.json`) parsing and validation.
//!
//! A V1 manifest is intentionally minimal:
//!
//! ```json
//! {
//!     "name": "colors",
//!     "version": "1.0.0",
//!     "description": "...",
//!     "main": "src/colors.eng",
//!     "license": "MIT",
//!     "dependencies": ["math"]
//! }
//! ```
//!
//! `name`, `version`, and `main` are required; the rest is optional.
//! Names must match a conservative regex so they can be safely used as
//! directory names and import targets.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::EnglingError;
use crate::package::version::{parse_version, Version};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub main: String,
    #[serde(default)]
    pub license: String,
    /// Optional list of package names this package depends on.
    /// V1 supports only flat names — no version specifiers.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Optional relative path to a checksum file (e.g. `SHA256SUMS`).
    /// The checksum is verified if the field is present in the
    /// manifest AND the archive provides a checksum entry; missing
    /// checksum files are tolerated for community packages.
    #[serde(default)]
    pub checksum: Option<String>,
    /// Optional explicit download URL for `eng update` to use when
    /// refreshing this package from a community URL install.
    #[serde(default)]
    pub update_url: Option<String>,
}

impl PackageManifest {
    /// Load a manifest from a JSON file on disk.
    pub fn from_file(path: &Path) -> Result<Self, EnglingError> {
        let text = fs::read_to_string(path).map_err(|e| {
            EnglingError::package(format!("could not read manifest {}: {e}", path.display()))
        })?;
        Self::from_json(&text)
    }

    /// Parse a manifest from a JSON string and validate it.
    pub fn from_json(text: &str) -> Result<Self, EnglingError> {
        let manifest: PackageManifest = serde_json::from_str(text)
            .map_err(|e| EnglingError::package(format!("manifest is invalid JSON: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Run all validation checks. Split out so that tests can drive a
    /// manifest that was just constructed in memory.
    pub fn validate(&self) -> Result<(), EnglingError> {
        validate_package_name(&self.name)?;
        if self.name.is_empty() {
            return Err(EnglingError::package("manifest is missing 'name'"));
        }
        if self.version.trim().is_empty() {
            return Err(EnglingError::package("manifest is missing 'version'"));
        }
        // Parsing is enough validation for V1 — it enforces the
        // major.minor.patch shape.
        parse_version(&self.version)?;
        if self.main.trim().is_empty() {
            return Err(EnglingError::package("manifest is missing 'main'"));
        }
        // Reject main paths that escape the package directory.
        if self
            .main
            .split('/')
            .any(|seg| seg == ".." || seg.is_empty() && self.main.contains(".."))
        {
            return Err(EnglingError::package(format!(
                "manifest 'main' path is invalid: {}",
                self.main
            )));
        }
        for dep in &self.dependencies {
            validate_package_name(dep).map_err(|_| {
                EnglingError::package(format!(
                    "dependency name '{dep}' is not a valid package identifier"
                ))
            })?;
        }
        Ok(())
    }

    pub fn parsed_version(&self) -> Result<Version, EnglingError> {
        parse_version(&self.version)
    }

    /// Write this manifest to a JSON file (used by the installer and by
    /// tests). Creates parent directories if needed.
    pub fn write_to(&self, path: &Path) -> Result<(), EnglingError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                EnglingError::package(format!(
                    "could not create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| EnglingError::package(format!("could not serialize manifest: {e}")))?;
        fs::write(path, text).map_err(|e| {
            EnglingError::package(format!("could not write manifest {}: {e}", path.display()))
        })
    }
}

/// Validate a package name. Used by both the manifest validator and by
/// the package name argument to `eng install`.
///
/// Rules:
///   * non-empty
///   * only lowercase ASCII letters, digits, `-`, `_`
///   * must start with a letter
///   * length 1..=64
pub fn validate_package_name(name: &str) -> Result<(), EnglingError> {
    if name.is_empty() {
        return Err(EnglingError::package("package name is empty"));
    }
    if name.len() > 64 {
        return Err(EnglingError::package(format!(
            "package name '{name}' is longer than 64 characters"
        )));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err(EnglingError::package(format!(
            "package name '{name}' must start with an ASCII letter"
        )));
    }
    for c in name.chars() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
        if !ok {
            return Err(EnglingError::package(format!(
                "package name '{name}' contains invalid character '{c}'"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let m = PackageManifest::from_json(
            r#"{
                "name": "colors",
                "version": "1.0.0",
                "main": "src/colors.eng"
            }"#,
        )
        .unwrap();
        assert_eq!(m.name, "colors");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.main, "src/colors.eng");
        assert_eq!(m.description, "");
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn parses_full_manifest() {
        let m = PackageManifest::from_json(
            r#"{
                "name": "math",
                "version": "0.2.3",
                "description": "Math helpers",
                "main": "src/main.eng",
                "license": "MIT",
                "dependencies": ["strings"]
            }"#,
        )
        .unwrap();
        assert_eq!(m.dependencies, vec!["strings".to_string()]);
    }

    #[test]
    fn rejects_invalid_name() {
        let err =
            PackageManifest::from_json(r#"{"name": "9bad", "version": "1.0.0", "main": "a.eng"}"#)
                .unwrap_err();
        assert!(format!("{err}").contains("package name"));
    }

    #[test]
    fn rejects_invalid_version() {
        let err =
            PackageManifest::from_json(r#"{"name": "good", "version": "1.0", "main": "a.eng"}"#)
                .unwrap_err();
        assert!(format!("{err}").contains("major.minor.patch"));
    }

    #[test]
    fn rejects_empty_main() {
        let err = PackageManifest::from_json(r#"{"name": "good", "version": "1.0.0", "main": ""}"#)
            .unwrap_err();
        assert!(format!("{err}").contains("main"));
    }

    #[test]
    fn rejects_path_traversal_main() {
        let err = PackageManifest::from_json(
            r#"{"name": "good", "version": "1.0.0", "main": "../etc/passwd"}"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("main"));
    }

    #[test]
    fn rejects_invalid_dependency_name() {
        let err = PackageManifest::from_json(
            r#"{
                "name": "good",
                "version": "1.0.0",
                "main": "a.eng",
                "dependencies": ["../bad"]
            }"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("dependency"));
    }

    #[test]
    fn name_validator_accepts_good_names() {
        for name in ["colors", "math-utils", "a", "math_utils", "x1"] {
            validate_package_name(name).unwrap();
        }
    }

    #[test]
    fn name_validator_rejects_garbage() {
        for name in [
            "",
            "9bad",
            "Has-Caps",
            "../escape",
            "name/with/slash",
            "x.y",
            "x y",
        ] {
            assert!(validate_package_name(name).is_err(), "should reject {name}");
        }
    }

    #[test]
    fn name_validator_rejects_too_long() {
        let name = "a".repeat(65);
        assert!(validate_package_name(&name).is_err());
    }
}
