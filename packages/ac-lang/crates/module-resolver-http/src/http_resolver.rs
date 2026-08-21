use std::path::Path;

use ac_dialect_canonical::AC_DIALECT;
use ac_fields::extract_fields;
use ac_frontend::frontend_for;
use ac_module_resolver_core::{ModuleCache, ModuleResolver, ResolvedModule};
use ac_module_resolver_noop::NoCache;

use crate::url::resolve_url;

/// HTTP-based module resolver.
///
/// Fetches documents via HTTP GET. Module paths must be valid URLs.
/// Relative paths are resolved against the base URL when provided.
///
/// By default, no module cache is used. Pass a [`ModuleCache`] to
/// [`HttpResolver::with_cache`] to memoize resolved modules by URL.
pub struct HttpResolver<C: ModuleCache = NoCache> {
    cache: C,
}

impl HttpResolver<NoCache> {
    /// Create a resolver with no caching.
    pub fn new() -> Self {
        HttpResolver { cache: NoCache }
    }
}

impl Default for HttpResolver<NoCache> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ModuleCache> HttpResolver<C> {
    /// Create a resolver with a specific cache implementation.
    pub fn with_cache(cache: C) -> Self {
        HttpResolver { cache }
    }
}

impl<C: ModuleCache> ModuleResolver for HttpResolver<C> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let url = resolve_url(path, base)?;
        let cache_key = url.to_string();

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached);
        }

        let content = ureq::get(url.as_str())
            .call()
            .ok()
            .and_then(|resp| resp.into_string().ok())?;

        let format = url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext))
            .unwrap_or("yaml");

        let frontend = frontend_for(format).ok()?;
        let value = frontend.parse(&content).ok()?;
        let extracted = extract_fields(value, &AC_DIALECT).ok()?;

        let module = ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            functions: extracted.functions,
            data: extracted.data.inner().clone(),
            base_path: None,
        };

        self.cache.insert(&cache_key, module.clone());
        Some(module)
    }
}
