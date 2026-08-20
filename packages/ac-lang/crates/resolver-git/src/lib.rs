//! # ac-resolver-git
//!
//! Git-based [`ModuleResolver`] implementation.
//!
//! Fetches documents from Git repositories using `git2`. Module paths use
//! the `git+` URL scheme:
//!
//! ```text
//! import('git+https://github.com/org/repo.git@main/schemas/base.yml')
//! ```
//!
//! Repositories are cloned to a cache directory keyed by a SHA-256 hash
//! of `(repo_url, ref)`. Clones are **bare** and **shallow** (depth=1 by
//! default) to minimize download size. Subsequent resolves from the same
//! repo+ref fetch updates from the remote before reading.

mod parse;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ac_fields::extract_fields;
use ac_frontend::frontend_for;
use ac_resolver_core::{ModuleCache, ModuleResolver, NoCache, ResolvedModule};
use sha2::{Digest, Sha256};

pub use parse::{parse_git_url, GitTarget};

/// Default shallow clone depth.
const DEFAULT_DEPTH: usize = 1;

/// Git-based module resolver.
///
/// Clones repositories to a cache and reads files from the Git object
/// store (not the working tree). Two levels of caching:
///
/// 1. **Repo cache** (internal) — keyed by `(repo_url, ref)`. Clones
///    once per repo+ref; subsequent files from the same repo read from
///    the cached clone after a lightweight fetch to refresh the ref.
/// 2. **Module cache** (via [`ModuleCache`]) — keyed by
///    `repo_url@ref/path`. Memoizes the extracted `ResolvedModule` so
///    the same file is parsed only once.
///
/// Clones are bare and shallow (depth configurable via [`depth`]).
/// Cache paths use SHA-256 for cross-version stability.
pub struct GitResolver<C: ModuleCache = NoCache> {
    cache_dir: PathBuf,
    module_cache: C,
    repo_cache: Mutex<HashMap<String, PathBuf>>,
    depth: usize,
}

impl GitResolver<NoCache> {
    /// Create a resolver with a specific cache directory, no module cache,
    /// and the default shallow clone depth (1).
    pub fn new(cache_dir: PathBuf) -> Self {
        GitResolver {
            cache_dir,
            module_cache: NoCache,
            repo_cache: Mutex::new(HashMap::new()),
            depth: DEFAULT_DEPTH,
        }
    }

    /// Create a resolver using the system temp directory and no module cache.
    pub fn temp() -> Self {
        let cache_dir = std::env::temp_dir().join("ac-resolver-git");
        GitResolver::new(cache_dir)
    }
}

impl<C: ModuleCache> GitResolver<C> {
    /// Create a resolver with a specific cache directory and module cache.
    pub fn with_cache(cache_dir: PathBuf, module_cache: C) -> Self {
        GitResolver {
            cache_dir,
            module_cache,
            repo_cache: Mutex::new(HashMap::new()),
            depth: DEFAULT_DEPTH,
        }
    }

    /// Create a resolver using the system temp directory and a module cache.
    pub fn temp_with_cache(module_cache: C) -> Self {
        let cache_dir = std::env::temp_dir().join("ac-resolver-git");
        GitResolver::with_cache(cache_dir, module_cache)
    }

    /// Set the shallow clone depth.
    ///
    /// A depth of `0` disables shallow cloning (full history). The default
    /// is `1` (only the latest commit).
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Compute the local cache path for a repo URL + ref using SHA-256.
    ///
    /// SHA-256 is stable across Rust versions, unlike `DefaultHasher`.
    fn cache_path(&self, target: &GitTarget) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(target.repo_url.as_bytes());
        hasher.update(b"\0");
        hasher.update(target.reference.as_bytes());
        let hash = hasher.finalize();
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
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
    fn fetch_file(&self, target: &GitTarget) -> Option<String> {
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

impl<C: ModuleCache> ModuleResolver for GitResolver<C> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let target = if path.starts_with("git+") {
            parse_git_url(path)?
        } else if let Some(base) = base {
            let base_str = base.to_str()?;
            if base_str.starts_with("git+") {
                let base_target = parse_git_url(base_str)?;
                let combined_path = format!("{}/{}", base_target.path, path);
                GitTarget {
                    repo_url: base_target.repo_url,
                    reference: base_target.reference,
                    path: combined_path,
                }
            } else {
                return None;
            }
        } else {
            return None;
        };

        let module_key = format!("{}@{}/{}", target.repo_url, target.reference, target.path);

        if let Some(cached) = self.module_cache.get(&module_key) {
            return Some(cached);
        }

        let content = self.fetch_file(&target)?;

        let format = target
            .path
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or("yaml");

        let frontend = frontend_for(format).ok()?;
        let value = frontend.parse(&content).ok()?;
        let extracted = extract_fields(value).ok()?;

        let base_path = format!(
            "git+{}@{}/{}",
            target.repo_url, target.reference, target.path
        );
        let base_path = PathBuf::from(base_path);

        let module = ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: Some(base_path),
        };

        self.module_cache.insert(&module_key, module.clone());
        Some(module)
    }
}

#[cfg(test)]
mod tests;
