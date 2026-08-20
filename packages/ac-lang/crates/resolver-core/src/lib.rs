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

/// A caching wrapper around any [`ModuleResolver`].
///
/// Memoizes `resolve` results by `(path, base)` key. Subsequent calls
/// with the same arguments return a clone of the cached result without
/// re-invoking the inner resolver. This eliminates redundant disk I/O,
/// parsing, and field extraction for repeated imports — including the
/// case where `ResolvePass` and `TypecheckPass` both resolve the same
/// import graph.
///
/// The cache uses `RefCell` for interior mutability so that `resolve`
/// remains `&self` (matching the [`ModuleResolver`] trait).
///
/// # Example
///
/// ```ignore
/// use ac_resolver_core::CachingResolver;
/// let resolver = CachingResolver::new(ac_resolver_file::FileResolver);
/// // First call reads from disk; second call hits the cache.
/// resolver.resolve("base.yml", None);
/// resolver.resolve("base.yml", None);
/// ```
pub struct CachingResolver<R: ModuleResolver> {
    inner: R,
    cache: RefCell<HashMap<(String, Option<PathBuf>), ResolvedModule>>,
}

impl<R: ModuleResolver> CachingResolver<R> {
    /// Wrap a resolver with a cache.
    pub fn new(inner: R) -> Self {
        CachingResolver {
            inner,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Clear the cache.
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Returns the number of cached entries.
    pub fn cache_len(&self) -> usize {
        self.cache.borrow().len()
    }
}

impl<R: ModuleResolver> ModuleResolver for CachingResolver<R> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let key = (path.to_string(), base.map(|p| p.to_path_buf()));
        if let Some(cached) = self.cache.borrow().get(&key) {
            return Some(cached.clone());
        }
        let result = self.inner.resolve(path, base);
        if let Some(ref module) = result {
            self.cache.borrow_mut().insert(key, module.clone());
        }
        result
    }
}
