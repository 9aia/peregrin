//! peregrin-bundle — the artifact model behind `peregrin install`
//! (ADR-0079). A *bundle* is a manifest (`peregrin-bundle.yaml`/`.yml`/
//! `.toml`) plus a payload of workflows, `.node.yml`, `.driver.yml`
//! and `.field.yml` files. This crate owns:
//!
//! - [`Manifest`] — the bundle manifest (name/version/`include:`).
//! - [`Store`] — `~/.local/share/peregrin/store/<name>@<version-or-sha>/`
//!   unpacked installs, each carrying a `.peregrin-store.json` record
//!   (source ref, resolved sha, content digest, installed-at).
//! - [`FetchCache`] — `~/.cache/peregrin/fetch/<key>/` content-addressed
//!   payloads; install is fetch→unpack so repeats and air-gapped nodes
//!   reuse cache entries.
//! - [`overlay`] — `include:` composition: base payloads merge under
//!   the local one (`deep_merge` semantics from the ported loader —
//!   local wins, arrays concat local-first — plus node-`id` dedupe so
//!   a redefined node doesn't double-fire).
//! - [`archive`] — `pack` (dir → `.tgz`) and tar/tgz unpack used by the
//!   tarball sources.
//! - [`digest`] — sha256 helpers the store's `verify` recomputes.

mod archive;
mod cache;
mod digest;
mod manifest;
mod overlay;
mod store;

use std::io;

pub use archive::{pack_tgz, untar, untar_gz};
pub use cache::FetchCache;
pub use digest::{dir_digest, dir_size, file_digest, sha256_hex};
pub use manifest::{MANIFEST_NAMES, Manifest};
pub use overlay::{overlay, payload_root};
pub use store::{RECORD_FILE, Store, StoreEntry, StoreRecord, VerifyReport, copy_tree};

/// Bundle failures — filesystem operations carry context; everything
/// else is a message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A filesystem operation failed.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },
    /// Manifest/lookup/archive validation failure.
    #[error("{0}")]
    Failed(String),
}

impl Error {
    /// Shorthand for [`Error::Io`].
    pub fn io(context: impl Into<String>) -> impl FnOnce(io::Error) -> Self {
        move |source| Self::Io {
            context: context.into(),
            source,
        }
    }
}
