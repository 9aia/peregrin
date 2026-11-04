//! The fetch cache — `~/.cache/peregrin/fetch/<key>/` content-addressed
//! payloads (`git-<sha>`, `sha256-<hex>`). Install is fetch→unpack, so
//! a warm cache makes repeat installs and air-gapped nodes free.
//! `remove` never touches it; eviction is by age ([`FetchCache::clean`])
//! or [`FetchCache::clean_all`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::Error;

/// Cache keys are `[A-Za-z0-9._-]+` — anything else can't be a dir
/// name we'd trust under the root anyway.
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && key != "."
        && key != ".."
}

/// The content-addressed fetch cache — one per root.
pub struct FetchCache {
    root: PathBuf,
}

impl FetchCache {
    /// A cache under `root` (created lazily on first commit).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The root dir.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path for a key — caller checks [`FetchCache::has`].
    #[must_use]
    pub fn entry(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }

    /// Whether a populated entry exists for `key`.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        valid_key(key) && self.entry(key).is_dir()
    }

    /// A scratch dir for an in-flight fetch — lives under the root so
    /// [`FetchCache::commit`] is a same-filesystem rename.
    pub fn scratch(&self) -> Result<PathBuf, Error> {
        fs::create_dir_all(&self.root)
            .map_err(Error::io(format!("create {}", self.root.display())))?;
        let scratch = self.root.join(format!(".scratch-{}", std::process::id()));
        if scratch.exists() {
            fs::remove_dir_all(&scratch)
                .map_err(Error::io(format!("clear {}", scratch.display())))?;
        }
        fs::create_dir_all(&scratch).map_err(Error::io(format!("create {}", scratch.display())))?;
        Ok(scratch)
    }

    /// Fold a filled scratch dir into `entry(key)` — an existing entry
    /// wins (same content by definition; the scratch is dropped).
    pub fn commit(&self, scratch: &Path, key: &str) -> Result<PathBuf, Error> {
        if !valid_key(key) {
            return Err(Error::Failed(format!("invalid cache key '{key}'")));
        }
        let dest = self.entry(key);
        if dest.exists() {
            fs::remove_dir_all(scratch)
                .map_err(Error::io(format!("clear {}", scratch.display())))?;
            return Ok(dest);
        }
        fs::rename(scratch, &dest).map_err(Error::io(format!(
            "commit {} → {}",
            scratch.display(),
            dest.display()
        )))?;
        Ok(dest)
    }

    /// Abandon a scratch dir after a failed fetch.
    pub fn discard(&self, scratch: &Path) {
        if scratch.exists() {
            let _ = fs::remove_dir_all(scratch);
        }
    }

    /// Entry dirs (dotfiles excluded — scratches aren't entries).
    pub fn entries(&self) -> Result<Vec<PathBuf>, Error> {
        let mut out = Vec::new();
        if self.root.is_dir() {
            let read = fs::read_dir(&self.root)
                .map_err(Error::io(format!("read {}", self.root.display())))?;
            for entry in read {
                let path = entry.map_err(Error::io("read cache entry"))?.path();
                let hidden = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'));
                if path.is_dir() && !hidden {
                    out.push(path);
                }
            }
        }
        out.sort();
        Ok(out)
    }

    /// Evict entries whose mtime is older than `max_age` —
    /// `Duration::ZERO` evicts everything.
    pub fn clean(&self, max_age: Duration) -> Result<Vec<PathBuf>, Error> {
        let now = SystemTime::now();
        let mut evicted = Vec::new();
        for path in self.entries()? {
            let stale = fs::metadata(&path)
                .and_then(|m| m.modified())
                .is_ok_and(|mtime| now.duration_since(mtime).is_ok_and(|age| age >= max_age));
            if stale {
                fs::remove_dir_all(&path)
                    .map_err(Error::io(format!("remove {}", path.display())))?;
                evicted.push(path);
            }
        }
        Ok(evicted)
    }

    /// Full eviction — `cache clean --all`.
    pub fn clean_all(&self) -> Result<Vec<PathBuf>, Error> {
        self.clean(Duration::ZERO)
    }
}
