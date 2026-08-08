//! Pluggable transport for fetching registry and package archives.
//!
//! Tests use [`LocalFetcher`], which serves files out of a directory
//! tree and never touches the network. The CLI wires up
//! [`HttpFetcher`], which delegates to the `ureq` HTTP client.
//!
//! `ureq` is configured (via the `rustls` feature) with the
//! `webpki-roots` CA bundle so HTTPS works out of the box without
//! pulling in native OpenSSL — that keeps the build friendly to
//! Termux / Android.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::PathBuf;
use std::time::Duration;

use ureq::tls::{RootCerts, TlsProvider};

use crate::error::EnglingError;

/// Anything that can produce a `Read`-able stream of bytes for `url`.
pub trait Fetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Box<dyn ReadSeek>, EnglingError>;
}

/// Stream of bytes plus the ability to seek back to the start. Returning
/// a single trait object that exposes both keeps the installer's checksum
/// code straight-forward: read everything into memory, hash, then seek
/// back and extract.
pub trait ReadSeek: Read + Send {}

impl<T> ReadSeek for T where T: Read + Send {}

/// Tests / air-gapped installs: serve files from a directory and
/// optional inline overrides.
#[derive(Debug, Default)]
pub struct LocalFetcher {
    pub root: Option<PathBuf>,
    /// Direct content overrides keyed by URL. Useful for tests that
    /// want to avoid touching the filesystem.
    pub inline: HashMap<String, Vec<u8>>,
}

impl LocalFetcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            inline: HashMap::new(),
        }
    }

    pub fn insert(&mut self, url: &str, data: impl Into<Vec<u8>>) {
        self.inline.insert(url.to_string(), data.into());
    }
}

impl Fetcher for LocalFetcher {
    fn fetch(&self, url: &str) -> Result<Box<dyn ReadSeek>, EnglingError> {
        if let Some(bytes) = self.inline.get(url) {
            return Ok(Box::new(Cursor::new(bytes.clone())));
        }
        if let Some(root) = &self.root {
            // Translate an https://... URL into a path under the root
            // by stripping scheme + host. This lets a test mock a
            // registry at `root/raw.githubusercontent.com/.../registry.json`.
            let relative = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .unwrap_or(url);
            let candidate = root.join(relative);
            if candidate.exists() {
                let file = File::open(&candidate).map_err(|e| {
                    EnglingError::package(format!("could not open {}: {e}", candidate.display()))
                })?;
                return Ok(Box::new(file));
            }
            return Err(EnglingError::package(format!(
                "could not download from {url}: no mock file at {}",
                candidate.display()
            )));
        }
        Err(EnglingError::package(format!(
            "could not download from {url}: no fetcher configured"
        )))
    }
}

/// Real network fetcher backed by `ureq` (Rustls + webpki-roots).
///
/// Both `http://` and `https://` URLs are supported. The HTTPS path
/// is the common case for the official registry and GitHub installs.
#[derive(Debug, Default, Clone)]
pub struct HttpFetcher {
    pub timeout_secs: u64,
}

impl HttpFetcher {
    pub fn new() -> Self {
        Self { timeout_secs: 60 }
    }

    /// Construct an `ureq` agent configured for the requested timeout.
    ///
    /// `http_status_as_error` is disabled so non-2xx responses still
    /// give us a readable body — useful for surfacing registry errors
    /// verbatim. The TLS provider is pinned to Rustls and the
    /// webpki-roots CA bundle is used unconditionally so HTTPS works
    /// out of the box without depending on the host's OpenSSL install
    /// (which is awkward on Termux / Android).
    fn agent(&self) -> ureq::Agent {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(self.timeout_secs)))
            .timeout_send_request(Some(Duration::from_secs(self.timeout_secs)))
            .timeout_recv_response(Some(Duration::from_secs(self.timeout_secs)))
            .timeout_recv_body(Some(Duration::from_secs(self.timeout_secs)))
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .provider(TlsProvider::Rustls)
                    .root_certs(RootCerts::WebPki)
                    .build(),
            )
            .http_status_as_error(false)
            .build();
        ureq::Agent::new_with_config(config)
    }

    fn http_get(&self, url: &str) -> Result<Vec<u8>, EnglingError> {
        let agent = self.agent();
        let response = agent
            .get(url)
            .header("User-Agent", "engling/1.0")
            .header("Accept", "*/*")
            .call();
        match response {
            Ok(mut resp) => {
                let status = resp.status().as_u16();
                let mut buf = Vec::new();
                if let Err(e) = resp.body_mut().as_reader().read_to_end(&mut buf) {
                    return Err(EnglingError::package(format!(
                        "could not read HTTP response from {url}: {e}"
                    )));
                }
                if !(200..300).contains(&status) {
                    let preview = String::from_utf8_lossy(&buf);
                    let preview = preview.chars().take(200).collect::<String>();
                    return Err(EnglingError::package(format!(
                        "could not download from {url}: HTTP {status}: {preview}"
                    )));
                }
                Ok(buf)
            }
            Err(e) => Err(EnglingError::package(format!(
                "could not download from {url}: {e}"
            ))),
        }
    }
}

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str) -> Result<Box<dyn ReadSeek>, EnglingError> {
        let bytes = self.http_get(url)?;
        Ok(Box::new(Cursor::new(bytes)))
    }
}

/// Helper used by tests to write a sample file under a temp directory.
pub fn write_file(path: &std::path::Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_fetcher_inline() {
        let mut f = LocalFetcher::new();
        f.insert("https://example.com/x", b"hello".to_vec());
        let mut s = f.fetch("https://example.com/x").unwrap();
        let mut buf = String::new();
        s.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "hello");
    }

    #[test]
    fn local_fetcher_root() {
        let dir = tempfile::tempdir().unwrap();
        // LocalFetcher maps http://host/path ↔ <root>/host/path by
        // stripping the scheme, so write the file at the URL-shaped path.
        let target = dir.path().join("example.com").join("data.bin");
        write_file(&target, b"abc").unwrap();
        let f = LocalFetcher::with_root(dir.path());
        let mut s = f.fetch("http://example.com/data.bin").unwrap();
        let mut buf = String::new();
        s.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "abc");
    }

    #[test]
    fn local_fetcher_missing() {
        let f = LocalFetcher::new();
        let res = f.fetch("https://example.com/nope");
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(format!("{err}").contains("no fetcher"));
    }

    #[test]
    fn http_fetcher_uses_rustls_with_webpki_roots() {
        // The fetcher MUST be configured for HTTPS. We verify this
        // out-of-band by inspecting the ureq agent's TLS config — no
        // network required, so the regression test is hermetic.
        let f = HttpFetcher::new();
        let agent = f.agent();
        let tls = agent.config().tls_config();
        assert_eq!(
            tls.provider(),
            TlsProvider::Rustls,
            "HttpFetcher must use Rustls, not native-tls / OpenSSL"
        );
        match tls.root_certs() {
            RootCerts::WebPki => {}
            other => panic!("expected webpki-roots CA bundle, got {other:?}"),
        }
        // HTTPS validation must NOT be disabled — that would let MITM
        // attacks slip through.
        assert!(!tls.disable_verification());
    }

    #[test]
    fn http_fetcher_supports_https_scheme() {
        // The fetcher must NOT short-circuit on https URLs anymore.
        // We assert this by giving it a non-routable host so the
        // request fails with a *network* error rather than the old
        // "HTTPS is not supported in this build" message.
        let f = HttpFetcher::new();
        // `https://0.0.0.0/` is reserved / non-routable; ureq will
        // either refuse to resolve it or fail to connect. Either
        // way, the failure must mention the URL, not "HTTPS is not
        // supported".
        let res = f.fetch("https://0.0.0.0./nope.bin");
        let err = match res {
            Ok(_) => panic!("expected fetch to fail for unroutable host"),
            Err(e) => format!("{e}"),
        };
        assert!(
            !err.contains("HTTPS is not supported"),
            "fetch should not reject HTTPS any more; got: {err}"
        );
    }
}
