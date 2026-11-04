//! Content digests — the store records `sha256` of the unpacked
//! payload so `peregrin verify` can recompute it, and unpinned fetches
//! (fs dirs, undigested tarballs) get their cache key from it.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{Error, store::RECORD_FILE};

/// sha256 hex of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// sha256 hex of a file's contents.
pub fn file_digest(path: &Path) -> Result<String, Error> {
    let bytes = fs::read(path).map_err(Error::io(format!("read {}", path.display())))?;
    Ok(sha256_hex(&bytes))
}

/// Sorted relative paths of payload files under `dir` — `.git/`
/// dirs and the root [`RECORD_FILE`] never count (the store writes
/// the record *after* taking the digest).
pub(crate) fn payload_files(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut out = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) -> Result<(), Error> {
    let entries = fs::read_dir(dir).map_err(Error::io(format!("read {}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(Error::io(format!("read {}", dir.display())))?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path).to_path_buf();
        let is_dir = path.is_dir();
        if is_dir {
            if rel == Path::new(".git") {
                continue;
            }
            walk(&path, base, out)?;
        } else if rel != Path::new(RECORD_FILE) {
            out.push(rel);
        }
    }
    Ok(())
}

/// sha256 hex over the sorted `relpath\0bytes` of every payload file —
/// order-independent, so reinstalls of identical trees match.
pub fn dir_digest(dir: &Path) -> Result<String, Error> {
    let mut hasher = Sha256::new();
    for rel in payload_files(dir)? {
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update([0]);
        let bytes = fs::read(dir.join(&rel))
            .map_err(Error::io(format!("read {}", dir.join(&rel).display())))?;
        hasher.update(bytes);
        hasher.update([0]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Total payload bytes — `peregrin info` size reporting.
pub fn dir_size(dir: &Path) -> Result<u64, Error> {
    let mut total = 0;
    for rel in payload_files(dir)? {
        total += fs::metadata(dir.join(&rel))
            .map_err(Error::io(format!("stat {}", dir.join(&rel).display())))?
            .len();
    }
    Ok(total)
}
