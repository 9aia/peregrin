//! peregrin-source-fs — the filepath arm of ADR-0079's ref grammar:
//! `./path`, `/path`, `~/path`, `nested/path`. A dir copies its
//! payload (`.git/` excluded); a `.tgz`/`.tar.gz`/`.tar` file unpacks.
//! The content key is the payload's sha256 — a repeat install of
//! identical bytes hits the fetch cache instead of re-copying.

use std::fs;
use std::path::Path;

use peregrin_bundle::{copy_tree, dir_digest, file_digest, untar, untar_gz};
use peregrin_source::{Error, Fetched, Resolved, Source, SourceRef, expand_path};

/// peregrin-bundle failures fold into the port's [`Error::Failed`].
fn be(e: &peregrin_bundle::Error) -> Error {
    Error::Failed(e.to_string())
}

/// [`Source`] over local paths.
#[derive(Clone, Copy, Debug, Default)]
pub struct FsSource;

impl FsSource {
    /// The fs source — stateless.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Source for FsSource {
    /// `~`-expand and existence-check the path; resolution is local so
    /// nothing is pinned here (the digest lands at fetch).
    fn resolve(&self, reference: &SourceRef) -> Result<Resolved, Error> {
        let SourceRef::Fs(path) = reference else {
            return Err(Error::Failed("FsSource only resolves fs refs".to_string()));
        };
        let path = expand_path(path);
        if !path.exists() {
            return Err(Error::Failed(format!("no such path: {}", path.display())));
        }
        Ok(Resolved::Fs { path })
    }

    /// Copy a dir or unpack a tarball into `dest_dir`.
    fn fetch(&self, resolved: &Resolved, dest_dir: &Path) -> Result<Fetched, Error> {
        let Resolved::Fs { path } = resolved else {
            return Err(Error::Failed("FsSource only fetches fs specs".to_string()));
        };
        fs::create_dir_all(dest_dir)
            .map_err(Error::io(format!("create {}", dest_dir.display())))?;
        let digest = if path.is_dir() {
            copy_tree(path, dest_dir).map_err(|e| be(&e))?;
            dir_digest(dest_dir).map_err(|e| be(&e))?
        } else {
            unpack_tarball(path, dest_dir)?
        };
        Ok(Fetched {
            dir: dest_dir.to_path_buf(),
            resolved: format!("sha256:{digest}"),
            content_key: format!("sha256-{digest}"),
        })
    }
}

/// A tarball *file* — digest the bytes for the key, then unpack by
/// extension (deliberately case-exact).
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn unpack_tarball(path: &Path, dest_dir: &Path) -> Result<String, Error> {
    let digest = file_digest(path).map_err(|e| be(&e))?;
    let file = fs::File::open(path).map_err(Error::io(format!("open {}", path.display())))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name.ends_with(".tgz") || name.ends_with(".tar.gz") {
        untar_gz(file, dest_dir).map_err(|e| be(&e))?;
    } else if name.ends_with(".tar") {
        untar(file, dest_dir).map_err(|e| be(&e))?;
    } else {
        return Err(Error::Failed(format!(
            "unsupported bundle file '{}' — expected a dir or .tgz/.tar.gz/.tar",
            path.display()
        )));
    }
    Ok(digest)
}
