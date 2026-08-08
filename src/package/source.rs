//! Detect what kind of source an `eng install <arg>` argument refers to.
//!
//! V1 supports three input shapes:
//!
//! 1. Bare package name — looked up in the official registry.
//! 2. A direct archive URL (typically ending in `.engpkg`) — downloaded
//!    and extracted directly.
//! 3. A Git repository URL — currently only GitHub HTTPS URLs are
//!    supported. The package manager fetches the repository archive
//!    from a GitHub Releases / codeload endpoint and treats the contents
//!    like a community `.engpkg`.
//!
//! Anything that looks URL-shaped but is missing a scheme is rejected
//! up front so users do not silently fall through to the registry.

use crate::error::EnglingError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    /// Bare package name → official registry lookup.
    OfficialName,
    /// Direct `.engpkg` archive URL.
    DirectArchive,
    /// GitHub HTTPS repository URL.
    GitHubRepo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSource {
    pub kind: SourceKind,
    /// The original argument as supplied on the command line.
    pub raw: String,
}

impl PackageSource {
    pub fn detect(arg: &str) -> Result<Self, EnglingError> {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            return Err(EnglingError::package(
                "install target is empty; provide a package name or URL",
            ));
        }

        if looks_like_url(trimmed) {
            let lower = trimmed.to_lowercase();
            if lower.ends_with(".engpkg") {
                return Ok(PackageSource {
                    kind: SourceKind::DirectArchive,
                    raw: trimmed.to_string(),
                });
            }
            if is_github_repo_url(trimmed) {
                return Ok(PackageSource {
                    kind: SourceKind::GitHubRepo,
                    raw: trimmed.to_string(),
                });
            }
            return Err(EnglingError::package(format!(
                "unsupported URL '{trimmed}': only GitHub repository URLs and *.engpkg archives are supported"
            )));
        }

        // Bare name. Validate as a package identifier so we don't end
        // up creating directories with `..` in them.
        crate::package::manifest::validate_package_name(trimmed)?;
        Ok(PackageSource {
            kind: SourceKind::OfficialName,
            raw: trimmed.to_string(),
        })
    }
}

/// Re-export of [`PackageSource::detect`] for callers that prefer a
/// free function over the inherent method.
pub fn detect_source_for(arg: &str) -> Result<PackageSource, EnglingError> {
    PackageSource::detect(arg)
}

/// Heuristic: anything that starts with `http://` or `https://` is a URL.
fn looks_like_url(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Whether the URL points at a GitHub repository (not a file, not a
/// gist, not a raw blob). We accept both `https://github.com/owner/repo`
/// and `https://github.com/owner/repo.git`.
pub fn is_github_repo_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    if !lower.starts_with("https://github.com/") && !lower.starts_with("http://github.com/") {
        return false;
    }
    let path = url
        .find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url)
        .trim_start_matches("github.com/")
        .trim_start_matches("www.github.com/");
    let mut parts = path.split('/').filter(|s| !s.is_empty());
    let owner = parts.next();
    let repo = parts.next();
    let trailing = parts.next();
    match (owner, repo) {
        (Some(o), Some(r)) => {
            let repo = r.trim_end_matches(".git");
            // Must be exactly /owner/repo — no trailing /blob/..., /tree/..., etc.
            !o.is_empty() && !repo.is_empty() && !repo.contains('/') && trailing.is_none()
        }
        _ => false,
    }
}

/// Parse a GitHub repository URL into `(owner, repo)`.
pub fn parse_github_repo(url: &str) -> Option<(String, String)> {
    if !is_github_repo_url(url) {
        return None;
    }
    let after = url
        .find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url)
        .trim_start_matches("github.com/")
        .trim_start_matches("www.github.com/");
    let mut parts = after.split('/').filter(|s| !s.is_empty());
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.trim_end_matches(".git").to_string();
    Some((owner, repo))
}

/// Construct the codeload archive URL for a GitHub repo.
/// `<owner>/<repo>` → `https://codeload.github.com/<owner>/<repo>/zip/refs/heads/<branch>`.
/// We use `main`, then `master` as a fallback branch name in [`fetch_github_zip_url`].
pub fn fetch_github_zip_url(owner: &str, repo: &str, branch: &str) -> String {
    format!("https://codeload.github.com/{owner}/{repo}/zip/refs/heads/{branch}")
}

/// Return a list of candidate branches to try, in priority order.
pub fn github_candidate_branches() -> &'static [&'static str] {
    &["main", "master"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_name_is_official() {
        let s = PackageSource::detect("colors").unwrap();
        assert_eq!(s.kind, SourceKind::OfficialName);
        assert_eq!(s.raw, "colors");
    }

    #[test]
    fn github_repo_url() {
        let s = PackageSource::detect("https://github.com/Alice/colors").unwrap();
        assert_eq!(s.kind, SourceKind::GitHubRepo);
    }

    #[test]
    fn github_repo_url_with_dot_git() {
        let s = PackageSource::detect("https://github.com/Alice/colors.git").unwrap();
        assert_eq!(s.kind, SourceKind::GitHubRepo);
    }

    #[test]
    fn direct_archive_url() {
        let s = PackageSource::detect("https://example.com/colors-1.0.0.engpkg").unwrap();
        assert_eq!(s.kind, SourceKind::DirectArchive);
    }

    #[test]
    fn rejects_unknown_url() {
        let err = PackageSource::detect("https://example.com/raw-file.txt").unwrap_err();
        assert!(format!("{err}").contains("unsupported URL"));
    }

    #[test]
    fn rejects_invalid_package_name() {
        let err = PackageSource::detect("../escape").unwrap_err();
        assert!(format!("{err}").contains("package name"));
    }

    #[test]
    fn rejects_empty_arg() {
        let err = PackageSource::detect("   ").unwrap_err();
        assert!(format!("{err}").contains("empty"));
    }

    #[test]
    fn parse_github_repo_extracts_owner_and_repo() {
        let (o, r) = parse_github_repo("https://github.com/Alice/colors").unwrap();
        assert_eq!(o, "Alice");
        assert_eq!(r, "colors");
        let (o, r) = parse_github_repo("https://github.com/Bob/widgets.git").unwrap();
        assert_eq!(o, "Bob");
        assert_eq!(r, "widgets");
    }

    #[test]
    fn is_github_repo_url_handles_edges() {
        assert!(is_github_repo_url("https://github.com/a/b"));
        assert!(is_github_repo_url("http://github.com/a/b.git"));
        assert!(!is_github_repo_url("https://github.com/a"));
        assert!(!is_github_repo_url(
            "https://github.com/a/b/blob/main/README.md"
        ));
        assert!(!is_github_repo_url("https://gitlab.com/a/b"));
    }
}
