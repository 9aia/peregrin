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
//! Repositories are cloned to a temp cache directory keyed by
//! `(repo_url, ref)`. Subsequent resolves from the same repo+ref read
//! from the cache without re-cloning.

mod parse;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ac_fields::extract_fields;
use ac_frontend::frontend_for;
use ac_resolver_core::{ModuleCache, ModuleResolver, NoCache, ResolvedModule};

pub use parse::{parse_git_url, GitTarget};

/// Git-based module resolver.
///
/// Clones repositories to a temp cache and reads files from the working
/// tree. Two levels of caching:
///
/// 1. **Repo cache** (internal) — keyed by `(repo_url, ref)`. Clones
///    once per repo+ref; subsequent files from the same repo read from
///    the cached clone.
/// 2. **Module cache** (via [`ModuleCache`]) — keyed by
///    `repo_url@ref/path`. Memoizes the extracted `ResolvedModule` so
///    the same file is parsed only once.
pub struct GitResolver<C: ModuleCache = NoCache> {
    cache_dir: PathBuf,
    module_cache: C,
    repo_cache: Mutex<HashMap<String, PathBuf>>,
}

impl GitResolver<NoCache> {
    /// Create a resolver with a specific cache directory and no module cache.
    pub fn new(cache_dir: PathBuf) -> Self {
        GitResolver {
            cache_dir,
            module_cache: NoCache,
            repo_cache: Mutex::new(HashMap::new()),
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
        }
    }

    /// Create a resolver using the system temp directory and a module cache.
    pub fn temp_with_cache(module_cache: C) -> Self {
        let cache_dir = std::env::temp_dir().join("ac-resolver-git");
        GitResolver::with_cache(cache_dir, module_cache)
    }

    /// Compute the local cache path for a repo URL + ref.
    fn cache_path(&self, target: &GitTarget) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hash;
        target.repo_url.hash(&mut hasher);
        target.reference.hash(&mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);
        self.cache_dir.join(format!("{hash:x}"))
    }

    /// Clone or open a repo, then read a file from it.
    fn fetch_file(&self, target: &GitTarget) -> Option<String> {
        let local = self.cache_path(target);

        if !local.exists() {
            std::fs::create_dir_all(&local).ok()?;
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(false);
            builder.clone(&target.repo_url, &local).ok()?;
        }

        let repo = git2::Repository::open(&local).ok()?;

        let refname = format!("refs/remotes/origin/{}", target.reference);
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
