use std::path::Path;

use crate::cache::{MemoryModuleCache, ModuleCache};
use crate::resolver::{ModuleResolver, ResolvedModule};

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
