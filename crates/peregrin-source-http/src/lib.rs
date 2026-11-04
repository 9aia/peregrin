//! peregrin-source-http — the tarball arm of ADR-0079's ref grammar:
//! `http(s)://….tgz|.tar.gz` GETs, verified against the optional
//! `#sha256:` pin, unpacked into the fetch dir. The tarball's sha256
//! is the content key (`sha256-<hex>`), so a pinned ref + warm cache
//! never touches the network — the air-gapped path.

use std::fs;
use std::path::Path;

use peregrin_bundle::{sha256_hex, untar_gz};
use peregrin_source::{Error, Fetched, Resolved, Source, SourceRef};

/// peregrin-bundle failures fold into the port's [`Error::Failed`].
fn be(e: &peregrin_bundle::Error) -> Error {
    Error::Failed(e.to_string())
}

/// Largest tarball a fetch buffers — 512 MiB.
const MAX_TARBALL: u64 = 512 * 1024 * 1024;

/// [`Source`] over http(s) tarballs.
pub struct HttpSource {
    agent: ureq::Agent,
}

impl HttpSource {
    /// An http source over a default ureq agent —
    /// `http_status_as_error(false)` so the status check below sees
    /// 4xx/5xx bodies instead of ureq's mapped error.
    #[must_use]
    pub fn new() -> Self {
        Self::with_agent(ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build(),
        ))
    }

    /// An http source over a custom agent (proxies, timeouts, fakes).
    #[must_use]
    pub fn with_agent(agent: ureq::Agent) -> Self {
        Self { agent }
    }
}

impl Default for HttpSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for HttpSource {
    /// The URL was already grammar-checked by
    /// [`peregrin_source::parse_ref`] — resolve only re-arms it.
    fn resolve(&self, reference: &SourceRef) -> Result<Resolved, Error> {
        let SourceRef::Http(http) = reference else {
            return Err(Error::Failed(
                "HttpSource only resolves http refs".to_string(),
            ));
        };
        Ok(Resolved::Http {
            url: http.url.clone(),
            digest: http.digest.clone(),
        })
    }

    /// GET → digest-check → unpack into `dest_dir`.
    fn fetch(&self, resolved: &Resolved, dest_dir: &Path) -> Result<Fetched, Error> {
        let Resolved::Http { url, digest } = resolved else {
            return Err(Error::Failed(
                "HttpSource only fetches http specs".to_string(),
            ));
        };
        fs::create_dir_all(dest_dir)
            .map_err(Error::io(format!("create {}", dest_dir.display())))?;
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| Error::Failed(format!("GET {url}: {e}")))?;
        if !response.status().is_success() {
            return Err(Error::Failed(format!(
                "GET {url}: HTTP {}",
                response.status()
            )));
        }
        let bytes = response
            .body_mut()
            .with_config()
            .limit(MAX_TARBALL)
            .read_to_vec()
            .map_err(|e| Error::Failed(format!("read {url}: {e}")))?;
        let hex = sha256_hex(&bytes);
        if let Some(want) = digest
            && hex != *want
        {
            return Err(Error::Failed(format!(
                "digest mismatch for {url}: sha256:{hex} ≠ pinned sha256:{want}"
            )));
        }
        untar_gz(bytes.as_slice(), dest_dir).map_err(|e| be(&e))?;
        Ok(Fetched {
            dir: dest_dir.to_path_buf(),
            resolved: format!("sha256:{hex}"),
            content_key: format!("sha256-{hex}"),
        })
    }
}
