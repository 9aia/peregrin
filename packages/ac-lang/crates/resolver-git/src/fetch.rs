use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::parse::GitTarget;

/// Default shallow clone depth.
pub const DEFAULT_DEPTH: usize = 1;

/// Git fetch helper — clones, opens, and reads files from a Git repository.
///
/// Encapsulates the low-level `git2` operations: cache path computation,
/// bare shallow cloning, ref refreshing, and blob reading.
pub struct GitFetcher {
    cache_dir: PathBuf,
    depth: usize,
}

impl GitFetcher {
    pub fn new(cache_dir: PathBuf, depth: usize) -> Self {
        GitFetcher { cache_dir, depth }
    }

    /// Compute the local cache path for a repo URL + ref using SHA-256.
    ///
    /// SHA-256 is stable across Rust versions, unlike `DefaultHasher`.
    pub fn cache_path(&self, target: &GitTarget) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(target.repo_url.as_bytes());
        hasher.update(b"\0");
        hasher.update(target.reference.as_bytes());
        let hash = hasher.finalize();
        let hex = hex::encode(hash);
        self.cache_dir.join(&hex[..16])
    }

    /// Build fetch options with the configured depth.
    fn fetch_options(&self) -> git2::FetchOptions<'_> {
        let mut fo = git2::FetchOptions::new();
        if self.depth > 0 {
            fo.depth(self.depth as i32);
        }
        fo
    }

    /// Clone or open a repo, then read a file from it.
    ///
    /// If the repo is already cached, performs a fetch to refresh the ref
    /// before reading. New clones are bare and shallow.
    pub fn fetch_file(&self, target: &GitTarget) -> Option<String> {
        let local = self.cache_path(target);

        if !local.exists() {
            std::fs::create_dir_all(&local).ok()?;
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(true);
            builder.fetch_options(self.fetch_options());
            builder.clone(&target.repo_url, &local).ok()?;
        }

        let repo = git2::Repository::open(&local).ok()?;

        // Refresh the ref from the remote to avoid stale data.
        let refname = format!("refs/remotes/origin/{}", target.reference);
        if let Ok(mut remote) = repo.find_remote("origin") {
            let _ = remote.fetch(
                &[&target.reference],
                Some(&mut self.fetch_options()),
                None,
            );
        }

        let oid = repo
            .revparse_single(&refname)
            .or_else(|_| repo.revparse_single(&target.reference))
            .ok()?
            .id();

        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;

        let entry = tree.get_path(Path::new(&target.path)).ok()?;
        let blob = repo.find_blob(entry.id()).ok()?;
        let content = std::str::from_utf8(blob.content()).ok()?;
        Some(content.to_string())
    }
}
