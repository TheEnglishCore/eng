//! Lightweight semver parser for V1 package versions.
//!
//! Supports `major.minor.patch` and comparison only. We deliberately do
//! not implement pre-release tags, build metadata, or ranges — the V1
//! dependency policy is "exact match" so we only need ordering.

use std::cmp::Ordering;
use std::fmt;

use crate::error::EnglingError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parse a `major.minor.patch` version string. Trailing components after
/// `patch` are not accepted for V1 — any pre-release/build suffix
/// produces an error so we never silently install an incompatible build.
pub fn parse_version(input: &str) -> Result<Version, EnglingError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(EnglingError::package("version is empty"));
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 3 {
        return Err(EnglingError::package(format!(
            "version '{input}' is not in major.minor.patch form"
        )));
    }
    let parse_component = |raw: &str| -> Result<u64, EnglingError> {
        if raw.is_empty() {
            return Err(EnglingError::package(format!(
                "version '{input}' contains an empty component"
            )));
        }
        // Reject leading + or - to keep things deterministic.
        if !raw.chars().all(|c| c.is_ascii_digit()) {
            return Err(EnglingError::package(format!(
                "version '{input}' contains non-digit characters"
            )));
        }
        raw.parse::<u64>().map_err(|_| {
            EnglingError::package(format!("version '{input}' has an overflowing component"))
        })
    };
    Ok(Version {
        major: parse_component(parts[0])?,
        minor: parse_component(parts[1])?,
        patch: parse_component(parts[2])?,
    })
}

/// Compare two versions in the natural semver order.
pub fn compare_versions(a: &Version, b: &Version) -> Ordering {
    a.major
        .cmp(&b.major)
        .then_with(|| a.minor.cmp(&b.minor))
        .then_with(|| a.patch.cmp(&b.patch))
}
