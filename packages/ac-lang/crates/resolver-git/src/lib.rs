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

use std::path::{Path, PathBuf};

use ac_fields::extract_fields;
use ac_frontend::frontend_for;
use ac_resolver_core::{ModuleResolver, ResolvedModule};

pub use parse::{parse_git_url, GitTarget};

/// Git-based module resolver.
///
/// Clones repositories to a temp cache and reads files from the working
/// tree. Cached by `(repo_url, ref)` to avoid re-cloning.
pub struct GitResolver {
    cache_dir: PathBuf,
}

impl GitResolver {
    /// Create a resolver with a specific cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        GitResolver { cache_dir }
    }

    /// Create a resolver using the system temp directory.
    pub fn temp() -> Self {
        let cache_dir = std::env::temp_dir().join("ac-resolver-git");
        GitResolver { cache_dir }
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

impl ModuleResolver for GitResolver {
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

        Some(ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: Some(base_path),
        })
    }
}

#[cfg(test)]
mod tests;
