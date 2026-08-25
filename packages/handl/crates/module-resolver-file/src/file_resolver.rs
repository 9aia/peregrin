use std::path::{Path, PathBuf};

use handl_dialect_canonical::AC_DIALECT;
use handl_fields::extract_fields;
use handl_frontend::frontend_for;
use handl_module_resolver_core::{ModuleCache, ModuleResolver, ResolvedModule};
use handl_module_resolver_noop::NoCache;

/// Filesystem-based module resolver.
///
/// Reads files from disk, parses by extension, extracts fields.
/// This is the default resolver for CLI usage.
///
/// By default, no module cache is used. Pass a [`ModuleCache`] to
/// [`FileResolver::with_cache`] to memoize resolved modules by
/// canonical path.
pub struct FileResolver<C: ModuleCache = NoCache> {
    cache: C,
}

impl FileResolver<NoCache> {
    /// Create a resolver with no caching.
    pub fn new() -> Self {
        FileResolver { cache: NoCache }
    }
}

impl Default for FileResolver<NoCache> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ModuleCache> FileResolver<C> {
    /// Create a resolver with a specific cache implementation.
    pub fn with_cache(cache: C) -> Self {
        FileResolver { cache }
    }
}

impl<C: ModuleCache> ModuleResolver for FileResolver<C> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let resolved = match base {
            Some(base) => base.join(path),
            None => PathBuf::from(path),
        };

        let canonical = resolved.canonicalize().unwrap_or(resolved.clone());
        let cache_key = canonical.to_string_lossy().to_string();

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached);
        }

        let content = std::fs::read_to_string(&resolved).ok()?;

        let format = resolved
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");

        let frontend = frontend_for(format).ok()?;
        let value = frontend.parse(content.as_str()).ok()?;
        let extracted = extract_fields(value, &AC_DIALECT).ok()?;

        let module = ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: resolved.parent().map(|p| p.to_path_buf()),
        };

        self.cache.insert(&cache_key, module.clone());
        Some(module)
    }
}
