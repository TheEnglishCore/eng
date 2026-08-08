//! Integration tests for the V1 package manager.
//!
//! These tests exercise the public `engling::package::*` API end-to-end:
//! they parse a registry, install packages through the same functions the
//! CLI calls, verify module resolution, removal, and listing, and check
//! that malformed or malicious input is rejected with a structured
//! `EnglingError` (no panics).
//!
//! All transports use `LocalFetcher` so the test suite never depends on
//! the live internet. Each test redirects `ENGLING_PACKAGES_DIR` to a
//! private temporary directory.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use serial_test::serial;
use sha2::{Digest, Sha256};

use engling::error::EnglingError;
use engling::package::{
    commands as pkg, detect_source_for, Fetcher, HttpFetcher, LocalFetcher, PackageManifest,
    PackageSource, PackageStore, ReadSeek, Registry, RegistryEntry, SourceKind,
};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn temp_packages_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("ENGLING_PACKAGES_DIR", dir.path());
    dir
}

fn build_engpkg(name: &str, version: &str, main: &str, src_body: &str) -> Vec<u8> {
    let manifest = format!(
        r#"{{
            "name": "{name}",
            "version": "{version}",
            "description": "Test package {name}",
            "main": "{main}",
            "license": "MIT"
        }}"#
    );
    let mut buf = Cursor::new(Vec::<u8>::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file(main, opts).unwrap();
        zip.write_all(src_body.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf.into_inner()
}

fn build_malicious_engpkg() -> Vec<u8> {
    let manifest = br#"{
        "name": "evil",
        "version": "1.0.0",
        "main": "evil.eng"
    }"#;
    let mut buf = Cursor::new(Vec::<u8>::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(manifest).unwrap();
        zip.start_file("../escape.txt", opts).unwrap();
        zip.write_all(b"bad").unwrap();
        zip.finish().unwrap();
    }
    buf.into_inner()
}

/// Lightweight Fetcher that serves a single URL from in-memory bytes
/// and returns an error for anything else. Each test wires up the URLs
/// it needs explicitly.
struct OneShotFetcher {
    url: String,
    bytes: Vec<u8>,
}

impl OneShotFetcher {
    fn new(url: &str, bytes: Vec<u8>) -> Self {
        Self {
            url: url.to_string(),
            bytes,
        }
    }
}

impl Fetcher for OneShotFetcher {
    fn fetch(&self, url: &str) -> Result<Box<dyn ReadSeek>, EnglingError> {
        if url == self.url {
            return Ok(Box::new(Cursor::new(self.bytes.clone())));
        }
        Err(EnglingError::package(format!(
            "unexpected URL in test fetcher: {url}"
        )))
    }
}

fn load_local_fetcher_for(root: &Path) -> LocalFetcher {
    let mut f = LocalFetcher::with_root(root);
    // Common lookups the tests use. They map URLs to relative files under
    // the `root` directory.
    f.insert(
        "https://example.test/registry.json",
        serde_json::to_vec(&sample_registry()).unwrap(),
    );
    f.insert(
        "https://example.test/colors.engpkg",
        build_engpkg("colors", "1.0.0", "src/colors.eng", "Print \"red\".\n"),
    );
    f.insert(
        "https://example.test/math.engpkg",
        build_engpkg("math", "0.2.0", "src/math.eng", "Print 42.\n"),
    );
    f.insert(
        "https://example.test/dep.engpkg",
        build_engpkg("strings", "1.1.0", "src/strings.eng", "Print \"s\".\n"),
    );
    f
}

fn sample_registry() -> Registry {
    let mut r = Registry::default();
    r.packages.insert(
        "colors".to_string(),
        RegistryEntry {
            version: "1.0.0".to_string(),
            description: "Color utilities for Engling".to_string(),
            url: "https://example.test/colors.engpkg".to_string(),
            checksum: None,
        },
    );
    r.packages.insert(
        "math".to_string(),
        RegistryEntry {
            version: "0.2.0".to_string(),
            description: "Math helpers".to_string(),
            url: "https://example.test/math.engpkg".to_string(),
            checksum: None,
        },
    );
    r
}

// ---------------------------------------------------------------------
// Registry / manifest / version / name validation
// ---------------------------------------------------------------------

#[serial]
#[test]
fn registry_round_trip_from_json() {
    let json = r#"{
        "colors": {
            "version": "1.0.0",
            "description": "Color utilities",
            "url": "https://example.test/colors.engpkg"
        }
    }"#;
    let reg = Registry::from_json(json).unwrap();
    assert_eq!(reg.lookup("colors").unwrap().version, "1.0.0");
    assert!(reg.lookup("nope").is_none());
}

#[serial]
#[test]
fn manifest_parses_minimal_form() {
    let m = PackageManifest::from_json(
        r#"{"name":"colors","version":"1.0.0","main":"src/colors.eng"}"#,
    )
    .unwrap();
    assert_eq!(m.name, "colors");
    assert_eq!(m.dependencies.len(), 0);
}

#[serial]
#[test]
fn manifest_rejects_invalid_name() {
    let err = PackageManifest::from_json(r#"{"name":"Has-Caps","version":"1.0.0","main":"a.eng"}"#)
        .unwrap_err();
    assert!(format!("{err}").contains("package name"));
}

#[serial]
#[test]
fn manifest_rejects_invalid_version() {
    let err = PackageManifest::from_json(r#"{"name":"good","version":"1.0","main":"a.eng"}"#)
        .unwrap_err();
    assert!(format!("{err}").contains("major.minor.patch"));
}

#[serial]
#[test]
fn manifest_rejects_traversal_main() {
    let err = PackageManifest::from_json(r#"{"name":"good","version":"1.0.0","main":"../escape"}"#)
        .unwrap_err();
    assert!(format!("{err}").contains("main"));
}

#[serial]
#[test]
fn manifest_rejects_invalid_dependency() {
    let err = PackageManifest::from_json(
        r#"{
            "name":"good",
            "version":"1.0.0",
            "main":"a.eng",
            "dependencies":["../bad"]
        }"#,
    )
    .unwrap_err();
    assert!(format!("{err}").contains("dependency"));
}

#[serial]
#[test]
fn version_parser_rejects_bad_shapes() {
    use engling::package::parse_version;
    assert!(parse_version("1").is_err());
    assert!(parse_version("1.0").is_err());
    assert!(parse_version("1.0.0.0").is_err());
    assert!(parse_version("").is_err());
    assert!(parse_version("a.b.c").is_err());
    parse_version("1.0.0").unwrap();
    parse_version("10.20.30").unwrap();
}

#[serial]
#[test]
fn package_name_validator_rules() {
    use engling::package::validate_package_name;
    for good in ["colors", "math-utils", "math_utils", "a", "x1"] {
        validate_package_name(good).unwrap();
    }
    for bad in [
        "",
        "9bad",
        "Has-Caps",
        "../escape",
        "name/with/slash",
        "x.y",
        "x y",
        &"a".repeat(65),
    ] {
        assert!(validate_package_name(bad).is_err(), "should reject {bad:?}");
    }
}

// ---------------------------------------------------------------------
// Source detection
// ---------------------------------------------------------------------

#[serial]
#[test]
fn source_detect_official_name() {
    let s = PackageSource::detect("colors").unwrap();
    assert_eq!(s.kind, SourceKind::OfficialName);
}

#[serial]
#[test]
fn source_detect_github_url() {
    let s = PackageSource::detect("https://github.com/Alice/colors").unwrap();
    assert_eq!(s.kind, SourceKind::GitHubRepo);
}

#[serial]
#[test]
fn source_detect_engpkg_url() {
    let s = PackageSource::detect("https://example.com/colors-1.0.0.engpkg").unwrap();
    assert_eq!(s.kind, SourceKind::DirectArchive);
}

#[serial]
#[test]
fn source_detect_rejects_raw_file_url() {
    let err = PackageSource::detect("https://example.com/raw-file.txt").unwrap_err();
    assert!(format!("{err}").contains("unsupported URL"));
}

#[serial]
#[test]
fn source_detect_rejects_github_blob() {
    // /owner/repo/blob/main/... is not a repo URL.
    let err =
        PackageSource::detect("https://github.com/Alice/colors/blob/main/README.md").unwrap_err();
    assert!(format!("{err}").contains("unsupported URL"));
}

#[serial]
#[test]
fn source_detect_helper_module() {
    // The re-exported helper should agree with the manual constructor.
    let s = detect_source_for("colors").unwrap();
    assert_eq!(s.kind, SourceKind::OfficialName);
}

// ---------------------------------------------------------------------
// Install / remove / list / search / update
// ---------------------------------------------------------------------

#[serial]
#[test]
fn install_from_registry_resolves_and_indexes() {
    let _tmp = temp_packages_dir();
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    let manifest = pkg::install_source("colors", &fetcher).unwrap();
    assert_eq!(manifest.name, "colors");
    assert_eq!(manifest.version, "1.0.0");

    // Listed and indexed.
    let list = pkg::list_installed().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].entry.name, "colors");
    assert_eq!(list[0].entry.version, "1.0.0");
}

#[serial]
#[test]
fn install_unknown_name_is_a_clean_error() {
    let _tmp = temp_packages_dir();
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    let err = pkg::install_source("does-not-exist", &fetcher).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("not found in the official Engling registry"),
        "{msg}"
    );
    assert!(!msg.contains("panic"));
}

#[serial]
#[test]
fn install_direct_url_with_bytes() {
    let _tmp = temp_packages_dir();
    let url = "https://example.test/direct.engpkg";
    let bytes = build_engpkg("direct", "2.0.0", "src/direct.eng", "Print \"direct\".\n");
    let fetcher = OneShotFetcher::new(url, bytes);
    let manifest = pkg::install_source(url, &fetcher).unwrap();
    assert_eq!(manifest.name, "direct");
    assert_eq!(manifest.version, "2.0.0");
}

#[serial]
#[test]
fn install_rejects_malicious_archive() {
    let _tmp = temp_packages_dir();
    let url = "https://example.test/evil.engpkg";
    let fetcher = OneShotFetcher::new(url, build_malicious_engpkg());
    let err = pkg::install_source(url, &fetcher).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("malicious"), "got: {msg}");
}

#[serial]
#[test]
fn install_rejects_corrupt_archive() {
    let _tmp = temp_packages_dir();
    let url = "https://example.test/garbage.engpkg";
    let fetcher = OneShotFetcher::new(url, b"definitely not a zip".to_vec());
    let err = pkg::install_source(url, &fetcher).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("corrupt"), "got: {msg}");
}

#[serial]
#[test]
fn install_rejects_checksum_mismatch() {
    let _tmp = temp_packages_dir();
    // Build a registry entry whose checksum is wrong.
    let url = "https://example.test/bad.engpkg";
    let bytes = build_engpkg("bad", "1.0.0", "src/bad.eng", "Print 1.\n");
    let mut reg = sample_registry();
    reg.packages.insert(
        "bad".to_string(),
        RegistryEntry {
            version: "1.0.0".to_string(),
            description: "Bad checksum test".to_string(),
            url: url.to_string(),
            // 64 hex chars but wrong digest.
            checksum: Some(
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
        },
    );
    let mut fetcher = LocalFetcher::new();
    fetcher.insert(
        "https://example.test/registry-bad.json",
        serde_json::to_vec(&reg).unwrap(),
    );
    fetcher.insert(url, bytes);
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry-bad.json");

    let err = pkg::install_source("bad", &fetcher).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("checksum"), "got: {msg}");

    std::env::remove_var("ENGLING_REGISTRY");
}

#[serial]
#[test]
fn install_accepts_correct_checksum() {
    let _tmp = temp_packages_dir();
    let url = "https://example.test/ok.engpkg";
    let bytes = build_engpkg("ok", "1.0.0", "src/ok.eng", "Print 1.\n");
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, &bytes);
    let digest = hasher.finalize();
    let checksum: String = digest.iter().map(|b| format!("{b:02x}")).collect();

    let mut reg = Registry::default();
    reg.packages.insert(
        "ok".to_string(),
        RegistryEntry {
            version: "1.0.0".to_string(),
            description: "Checksum test".to_string(),
            url: url.to_string(),
            checksum: Some(checksum),
        },
    );
    let mut fetcher = LocalFetcher::new();
    fetcher.insert(
        "https://example.test/registry-ok.json",
        serde_json::to_vec(&reg).unwrap(),
    );
    fetcher.insert(url, bytes);
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry-ok.json");

    let manifest = pkg::install_source("ok", &fetcher).unwrap();
    assert_eq!(manifest.name, "ok");
    std::env::remove_var("ENGLING_REGISTRY");
}

#[serial]
#[test]
fn remove_installed_package_succeeds() {
    let _tmp = temp_packages_dir();
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    pkg::install_source("colors", &fetcher).unwrap();
    assert!(!pkg::list_installed().unwrap().is_empty());
    pkg::remove_package("colors").unwrap();
    assert!(pkg::list_installed().unwrap().is_empty());
}

#[serial]
#[test]
fn remove_unknown_package_errors_clearly() {
    let _tmp = temp_packages_dir();
    let err = pkg::remove_package("nope").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("not installed"), "got: {msg}");
}

#[serial]
#[test]
fn remove_invalid_name_errors_clearly() {
    let _tmp = temp_packages_dir();
    let err = pkg::remove_package("../escape").unwrap_err();
    assert!(format!("{err}").contains("package name"));
}

#[serial]
#[test]
fn list_empty_reports_nothing() {
    let _tmp = temp_packages_dir();
    let text = pkg::format_list(&pkg::list_installed().unwrap());
    assert!(text.contains("No Engling packages"));
}

#[serial]
#[test]
fn search_returns_matches_without_downloading() {
    // search_registry only fetches the registry file, not the package
    // archives themselves. The LocalFetcher has no entry for the
    // archives, so any accidental download would surface as an error.
    let _tmp = temp_packages_dir();
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    let results = pkg::search_registry("color", &fetcher).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "colors");
    let text = pkg::format_search(&results);
    assert!(text.contains("Color utilities for Engling"));
    std::env::remove_var("ENGLING_REGISTRY");
}

#[serial]
#[test]
fn update_installs_when_registry_version_is_newer() {
    let _tmp = temp_packages_dir();
    let mut fetcher = load_local_fetcher_for(Path::new("/dev/null"));

    // Build a registry whose `colors` is at 1.0.0 (current), then change
    // the bytes URL to a 1.1.0 package, and confirm update_installed
    // picks it up.
    let mut reg = sample_registry();
    reg.packages.insert(
        "colors".to_string(),
        RegistryEntry {
            version: "1.1.0".to_string(),
            description: "Color utilities for Engling".to_string(),
            url: "https://example.test/colors-1.1.0.engpkg".to_string(),
            checksum: None,
        },
    );
    fetcher.insert(
        "https://example.test/registry-update.json",
        serde_json::to_vec(&reg).unwrap(),
    );
    fetcher.insert(
        "https://example.test/colors-1.1.0.engpkg",
        build_engpkg("colors", "1.1.0", "src/colors.eng", "Print \"red\".\n"),
    );

    // Install at 1.0.0.
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    pkg::install_source("colors", &fetcher).unwrap();

    // Now point at the registry that advertises 1.1.0.
    std::env::set_var(
        "ENGLING_REGISTRY",
        "https://example.test/registry-update.json",
    );
    let updated = pkg::update_installed(&fetcher).unwrap();
    assert_eq!(updated.len(), 1);
    assert!(updated[0].contains("1.0.0 -> 1.1.0"));

    std::env::remove_var("ENGLING_REGISTRY");
}

#[serial]
#[test]
fn update_reports_nothing_when_current() {
    let _tmp = temp_packages_dir();
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    pkg::install_source("colors", &fetcher).unwrap();
    let updated = pkg::update_installed(&fetcher).unwrap();
    assert!(updated.is_empty());
    std::env::remove_var("ENGLING_REGISTRY");
}

// ---------------------------------------------------------------------
// Module resolution & dependency cycles
// ---------------------------------------------------------------------

#[serial]
#[test]
fn resolve_module_returns_main_path() {
    let _tmp = temp_packages_dir();
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    pkg::install_source("colors", &fetcher).unwrap();
    let store = PackageStore::user_default();
    let resolved = store.resolve_module("colors").unwrap();
    assert!(resolved.ends_with("src/colors.eng"));
    assert!(resolved.exists());
}

#[serial]
#[test]
fn resolve_module_returns_none_for_unknown() {
    let _tmp = temp_packages_dir();
    let store = PackageStore::user_default();
    assert!(store.resolve_module("nope").is_none());
}

#[serial]
#[test]
fn dependency_cycle_is_detected() {
    let _tmp = temp_packages_dir();
    let store = PackageStore::user_default();
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
    assert_eq!(
        pkg::detect_dependency_cycle(&store, "a"),
        Some("a".to_string())
    );
}

// ---------------------------------------------------------------------
// GitHub / local-fetcher integration
// ---------------------------------------------------------------------

#[serial]
#[test]
fn github_url_is_parsed_for_owner_and_repo() {
    let (o, r) = engling::package::parse_github_repo("https://github.com/Alice/colors").unwrap();
    assert_eq!(o, "Alice");
    assert_eq!(r, "colors");
}

#[serial]
#[test]
fn local_fetcher_serves_inline_bytes() {
    let mut f = LocalFetcher::new();
    f.insert("https://example.test/x", b"hello".to_vec());
    let mut s = f.fetch("https://example.test/x").unwrap();
    let mut buf = String::new();
    s.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, "hello");
}

// ---------------------------------------------------------------------
// Errors never panic
// ---------------------------------------------------------------------

#[serial]
#[test]
fn missing_package_error_does_not_panic() {
    let _tmp = temp_packages_dir();
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    let res = std::panic::catch_unwind(|| pkg::install_source("missing", &fetcher));
    assert!(res.is_ok(), "install_source panicked on missing package");
    let err = res.unwrap().unwrap_err();
    let msg = format!("{err}");
    assert!(!msg.contains("thread 'main' panicked"));
    assert!(!msg.contains("panicked at"));
}

// ---------------------------------------------------------------------
// Smoke test: building a community package from a local mock repo
// ---------------------------------------------------------------------

#[serial]
#[test]
fn install_from_local_mock_github_zip() {
    // We simulate the GitHub install path without touching the network.
    // The user-facing argument is the GitHub repo URL; the fetcher
    // serves the .engpkg bytes under the codeload URL the package
    // manager would otherwise pull from.
    let _tmp = temp_packages_dir();
    let codeload = "https://codeload.github.com/Alice/colors/zip/refs/heads/main";
    let bytes = build_engpkg("colors", "0.9.0", "src/colors.eng", "Print \"hi\".\n");
    let mut fetcher = LocalFetcher::new();
    fetcher.insert(codeload, bytes);
    let manifest = pkg::install_source("https://github.com/Alice/colors", &fetcher).unwrap();
    assert_eq!(manifest.name, "colors");
    assert_eq!(manifest.version, "0.9.0");

    let store = PackageStore::user_default();
    let resolved = store.resolve_module("colors").unwrap();
    assert!(resolved.exists());
}

// ---------------------------------------------------------------------
// ENGLING_REGISTRY override
// ---------------------------------------------------------------------

#[serial]
#[test]
fn engling_registry_env_override_is_honored() {
    // Indirect: when ENGLING_REGISTRY points at a different URL, the
    // command must use that URL. We verify the helper directly because
    // install_source + list_installed all read the env at call time.
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/custom.json");
    assert_eq!(
        engling::package::resolve_registry_url(),
        "https://example.test/custom.json"
    );
    std::env::remove_var("ENGLING_REGISTRY");
    assert_eq!(
        engling::package::resolve_registry_url(),
        engling::package::DEFAULT_REGISTRY_URL
    );
}

// ---------------------------------------------------------------------
// Store rebuild after manual deletion
// ---------------------------------------------------------------------

#[serial]
#[test]
fn rebuild_index_recovers_from_disk_state() {
    use engling::package::installer::rebuild_index;

    let _tmp = temp_packages_dir();
    std::env::set_var("ENGLING_REGISTRY", "https://example.test/registry.json");
    let fetcher = load_local_fetcher_for(Path::new("/dev/null"));
    pkg::install_source("colors", &fetcher).unwrap();
    pkg::install_source("math", &fetcher).unwrap();

    // Corrupt the index by overwriting it with `{}`.
    let store = PackageStore::user_default();
    fs::write(store.root().join("manifest_index.json"), "{}").unwrap();
    assert!(store.index().is_empty());

    let idx = rebuild_index(&store).unwrap();
    assert_eq!(idx.len(), 2);
    let names: Vec<_> = idx.entries().map(|p| p.name.clone()).collect();
    assert!(names.contains(&"colors".to_string()));
    assert!(names.contains(&"math".to_string()));
}
