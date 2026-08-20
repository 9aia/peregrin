//! # ac-resolver-core
//!
//! Module resolution abstraction — decoupled from typegen.
//!
//! Defines the [`ModuleResolver`] trait and [`ResolvedModule`] struct.
//! Implementations live in separate crates:
//!
//! - `ac-resolver-file` — filesystem-based (`std::fs`)
//! - `ac-resolver-virtual` — in-memory (`HashMap`)
//! - `ac-resolver-http` — HTTP(S) via `ureq`
//! - `ac-resolver-git` — Git repositories via `git2`
//!
//! Any system that needs to follow `import()` calls (typegen, validation,
//! linting) can take `&dyn ModuleResolver` without caring where modules
//! come from.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;

/// A module resolver — abstracts how import paths are loaded.
///
/// Implementations:
/// - `ac-resolver-file` — filesystem-based (`std::fs`)
/// - `ac-resolver-virtual` — in-memory (`HashMap`)
/// - `ac-resolver-http` — HTTP(S) via `ureq`
/// - `ac-resolver-git` — Git repositories via `git2`
pub trait ModuleResolver {
    /// Resolve a module path to its extracted fields + data.
    ///
    /// `base` is the base path of the importing document (for relative
    /// resolution). Returns `None` if the module cannot be found.
    fn resolve(&self, path: &str, base: Option<&std::path::Path>) -> Option<ResolvedModule>;
}

/// A resolved module — the result of a `ModuleResolver::resolve` call.
///
/// Contains everything needed to merge imported schemas into a parent
/// context and recurse into nested imports.
#[derive(Clone)]
pub struct ResolvedModule {
    /// Field name → raw schema fragment (AC keys still present).
    pub schemas: Vec<(String, JsonValue)>,
    /// Field names marked with `merge: true`.
    pub merge: Vec<String>,
    /// The document's data values (for data-driven import resolution).
    pub data: JsonValue,
    /// Base path for resolving nested imports from this module.
    pub base_path: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Module cache
// ---------------------------------------------------------------------------

/// Cache for resolved modules, keyed by a resolver-specific string.
///
/// Each resolver decides what to cache and how to key it:
/// - **FileResolver** — key = canonical path
/// - **HttpResolver** — key = full URL
/// - **GitResolver** — key = `repo_url@ref/path` (module-level),
///   plus an internal repo cache for clone-level dedup
///
/// Implementations must be thread-safe (`Send`) since resolvers may be
/// shared across passes (`ResolvePass` + `TypecheckPass`).
pub trait ModuleCache: Send {
    fn get(&self, key: &str) -> Option<ResolvedModule>;
    fn insert(&self, key: &str, module: ResolvedModule);
    fn clear(&self);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Default in-memory cache backed by a `RefCell<HashMap>`.
///
/// Suitable for single-threaded usage (the common pipeline case).
/// For multi-threaded scenarios, wrap with a `Mutex`-backed impl.
pub struct MemoryModuleCache {
    map: RefCell<HashMap<String, ResolvedModule>>,
}

impl MemoryModuleCache {
    pub fn new() -> Self {
        MemoryModuleCache {
            map: RefCell::new(HashMap::new()),
        }
    }
}

impl Default for MemoryModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleCache for MemoryModuleCache {
    fn get(&self, key: &str) -> Option<ResolvedModule> {
        self.map.borrow().get(key).cloned()
    }

    fn insert(&self, key: &str, module: ResolvedModule) {
        self.map.borrow_mut().insert(key.to_string(), module);
    }

    fn clear(&self) {
        self.map.borrow_mut().clear();
    }

    fn len(&self) -> usize {
        self.map.borrow().len()
    }
}

/// No-op cache — every `get` returns `None`.
///
/// Use when caching is not desired (e.g., tests that need fresh I/O
/// on every resolve, or memory-constrained environments).
pub struct NoCache;

impl ModuleCache for NoCache {
    fn get(&self, _key: &str) -> Option<ResolvedModule> {
        None
    }

    fn insert(&self, _key: &str, _module: ResolvedModule) {}

    fn clear(&self) {}

    fn len(&self) -> usize {
        0
    }
}

// ---------------------------------------------------------------------------
// CachingResolver — convenience wrapper
// ---------------------------------------------------------------------------

/// A caching wrapper around any [`ModuleResolver`].
///
/// Uses a [`ModuleCache`] to memoize `resolve` results. The cache key
/// is `path:base` (stringified). Subsequent calls with the same
/// arguments return a cached clone without re-invoking the inner
/// resolver.
///
/// For resolvers that need resolver-specific caching (e.g., git's
/// repo-level clone cache), prefer passing a `ModuleCache` directly
/// to the resolver instead of wrapping with `CachingResolver`.
///
/// # Example
///
/// ```ignore
/// use ac_resolver_core::{CachingResolver, MemoryModuleCache};
/// let resolver = CachingResolver::new(
///     ac_resolver_file::FileResolver::new(),
///     MemoryModuleCache::new(),
/// );
/// // First call reads from disk; second call hits the cache.
/// resolver.resolve("base.yml", None);
/// resolver.resolve("base.yml", None);
/// ```
pub struct CachingResolver<R: ModuleResolver, C: ModuleCache = MemoryModuleCache> {
    inner: R,
    cache: C,
}

impl<R: ModuleResolver> CachingResolver<R, MemoryModuleCache> {
    /// Wrap a resolver with a default in-memory cache.
    pub fn new(inner: R) -> Self {
        CachingResolver {
            inner,
            cache: MemoryModuleCache::new(),
        }
    }
}

impl<R: ModuleResolver, C: ModuleCache> CachingResolver<R, C> {
    /// Wrap a resolver with a specific cache implementation.
    pub fn with_cache(inner: R, cache: C) -> Self {
        CachingResolver { inner, cache }
    }

    /// Clear the cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Returns the number of cached entries.
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }
}

impl<R: ModuleResolver, C: ModuleCache> ModuleResolver for CachingResolver<R, C> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let key = match base {
            Some(b) => format!("{}:{}", path, b.display()),
            None => path.to_string(),
        };
        if let Some(cached) = self.cache.get(&key) {
            return Some(cached);
        }
        let result = self.inner.resolve(path, base);
        if let Some(ref module) = result {
            self.cache.insert(&key, module.clone());
        }
        result
    }
}
