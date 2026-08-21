use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ac_dialect_canonical::AC_DIALECT;
use ac_fields::extract_fields;
use ac_frontend::frontend_for;
use ac_module_resolver_core::{ModuleCache, ModuleResolver, ResolvedModule};
use ac_module_resolver_noop::NoCache;

use crate::parse::{parse_git_url, GitTarget};
use crate::fetch::{GitFetcher, DEFAULT_DEPTH};

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
    fetcher: GitFetcher,
}

impl GitResolver<NoCache> {
    /// Create a resolver with a specific cache directory, no module cache,
    /// and the default shallow clone depth (1).
    pub fn new(cache_dir: PathBuf) -> Self {
        GitResolver {
            cache_dir: cache_dir.clone(),
            module_cache: NoCache,
            repo_cache: Mutex::new(HashMap::new()),
            fetcher: GitFetcher::new(cache_dir, DEFAULT_DEPTH),
        }
    }

    /// Create a resolver using the system temp directory and no module cache.
    pub fn temp() -> Self {
        let cache_dir = std::env::temp_dir().join("ac-module-resolver-git");
        GitResolver::new(cache_dir)
    }
}

impl<C: ModuleCache> GitResolver<C> {
    /// Create a resolver with a specific cache directory and module cache.
    pub fn with_cache(cache_dir: PathBuf, module_cache: C) -> Self {
        GitResolver {
            cache_dir: cache_dir.clone(),
            module_cache,
            repo_cache: Mutex::new(HashMap::new()),
            fetcher: GitFetcher::new(cache_dir, DEFAULT_DEPTH),
        }
    }

    /// Create a resolver using the system temp directory and a module cache.
    pub fn temp_with_cache(module_cache: C) -> Self {
        let cache_dir = std::env::temp_dir().join("ac-module-resolver-git");
        GitResolver::with_cache(cache_dir, module_cache)
    }

    /// Set the shallow clone depth.
    ///
    /// A depth of `0` disables shallow cloning (full history). The default
    /// is `1` (only the latest commit).
    pub fn depth(mut self, depth: usize) -> Self {
        self.fetcher = GitFetcher::new(self.cache_dir.clone(), depth);
        self
    }
}

impl<C: ModuleCache> ModuleResolver for GitResolver<C> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let target = if path.starts_with("git+") {
            parse_git_url(path)?
        } else {
            let base = base?;
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
        };

        let module_key = format!("{}@{}/{}", target.repo_url, target.reference, target.path);
        let repo_key = format!("{}@{}", target.repo_url, target.reference);

        if let Some(cached) = self.module_cache.get(&module_key) {
            return Some(cached);
        }

        let local = self
            .repo_cache
            .lock()
            .unwrap()
            .get(&repo_key)
            .cloned();

        let content = match local {
            Some(path) => self.fetcher.fetch_file_at(&target, &path)?,
            None => {
                let path = self.fetcher.cache_path(&target);
                let content = self.fetcher.fetch_file_at(&target, &path)?;
                self.repo_cache
                    .lock()
                    .unwrap()
                    .insert(repo_key, path);
                content
            }
        };

        let format = target
            .path
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or("yaml");

        let frontend = frontend_for(format).ok()?;
        let value = frontend.parse(&content).ok()?;
        let extracted = extract_fields(value, &AC_DIALECT).ok()?;

        let base_path = format!(
            "git+{}@{}/{}",
            target.repo_url, target.reference, target.path
        );
        let base_path = PathBuf::from(base_path);

        let module = ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            functions: extracted.functions,
            data: extracted.data.inner().clone(),
            base_path: Some(base_path),
        };

        self.module_cache.insert(&module_key, module.clone());
        Some(module)
    }
}
