//! The base-dir manifest — `.peregrin-worktrees.json` records what
//! was made where, so `list`/`cleanup`/`keep`/`destroy` work across
//! processes and `list` can diff records against on-disk reality.
//! JSON (not a db): the file is the debug surface — a human can read
//! and hand-edit it, and a corrupt one errors loudly rather than
//! being rebuilt silently.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Worktree};

pub(crate) const MANIFEST_FILE: &str = ".peregrin-worktrees.json";

/// One manifest record — mirrors [`Worktree`], owned by the file.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub path: PathBuf,
    pub repo: PathBuf,
    pub sha: String,
    pub kept: bool,
}

impl Entry {
    pub fn to_worktree(&self) -> Worktree {
        Worktree {
            path: self.path.clone(),
            repo: self.repo.clone(),
            sha: self.sha.clone(),
            kept: self.kept,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub entries: Vec<Entry>,
}

impl Manifest {
    pub fn find(&self, path: &Path) -> Option<&Entry> {
        self.entries.iter().find(|e| e.path == path)
    }

    pub fn remove(&mut self, path: &Path) {
        self.entries.retain(|e| e.path != path);
    }

    /// Record a fresh worktree — callers reconcile first (`create`
    /// returns live records early), so `path` is never already here.
    pub fn record(&mut self, worktree: &Worktree) {
        self.entries.push(Entry {
            path: worktree.path.clone(),
            repo: worktree.repo.clone(),
            sha: worktree.sha.clone(),
            kept: worktree.kept,
        });
    }
}

/// Missing manifest = empty manifest (nothing made yet). A present
/// file that won't read or parse is a hard error — guessing at
/// tracked state would let `cleanup` miss live checkouts.
pub(crate) fn load(base: &Path) -> Result<Manifest, Error> {
    let file = base.join(MANIFEST_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(Manifest::default());
        }
        Err(source) => {
            return Err(Error::Io {
                context: format!("read {}", file.display()),
                source,
            });
        }
    };
    serde_json::from_str(&text).map_err(|source| Error::Manifest { path: file, source })
}

/// Create the base dir on first write, then serialize. Writes go
/// straight to the fd (no buffered writer swallowing a final-flush
/// error — `to_writer` reports it).
pub(crate) fn save(base: &Path, manifest: &Manifest) -> Result<(), Error> {
    fs::create_dir_all(base).map_err(|source| Error::Io {
        context: format!("create {}", base.display()),
        source,
    })?;
    let file = base.join(MANIFEST_FILE);
    let handle = fs::File::create(&file).map_err(|source| Error::Io {
        context: format!("create {}", file.display()),
        source,
    })?;
    serde_json::to_writer_pretty(&handle, manifest)
        .map_err(|source| Error::Manifest { path: file, source })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// `/dev/full` opens for write fine and fails on the serialize —
    /// the only way `to_writer`'s error arm is reachable, and only
    /// from inside the crate (every public op `load`s first).
    #[cfg(unix)]
    #[test]
    fn save_reports_write_failure() {
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/dev/full", dir.path().join(MANIFEST_FILE)).unwrap();
        let err = save(dir.path(), &Manifest::default()).unwrap_err();
        assert!(matches!(err, Error::Manifest { .. }));
    }
}
