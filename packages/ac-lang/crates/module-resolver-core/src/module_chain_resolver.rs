use std::path::Path;

use crate::module_resolver::ModuleResolver;
use crate::resolved_module::ResolvedModule;

/// A resolver that tries multiple resolvers in order, returning the
/// first non-`None` result.
///
/// This enables mixed import sources in a single document — e.g.,
/// local files, HTTP URLs, and git URLs — without the caller needing
/// to know which resolver handles which path.
///
/// Each resolver returns `None` for paths it doesn't handle, so the
/// chain falls through to the next. Resolvers that can quickly
/// determine a path is not theirs (e.g., `FileResolver` sees `git+`
/// prefix) should short-circuit early.
///
/// # Example
///
/// ```ignore
/// use ac_module_resolver_core::ChainResolver;
///
/// let resolver = ChainResolver::new(vec![
///     Box::new(ac_module_resolver_file::FileResolver::with_cache(
///     ac_module_resolver_memory::MemoryModuleCache::new(),
///     )),
///     Box::new(ac_module_resolver_git::GitResolver::temp_with_cache(
///         ac_module_resolver_memory::MemoryModuleCache::new(),
///     )),
///     Box::new(ac_module_resolver_http::HttpResolver::with_cache(
///         ac_module_resolver_memory::MemoryModuleCache::new(),
///     )),
/// ]);
/// ```
pub struct ChainResolver {
    resolvers: Vec<Box<dyn ModuleResolver>>}

impl ChainResolver {
    /// Create a chain from an ordered list of resolvers.
    ///
    /// Resolvers are tried in the order provided. The first to return
    /// a `Some` result wins.
    pub fn new(resolvers: Vec<Box<dyn ModuleResolver>>) -> Self {
        ChainResolver { resolvers }
    }
}

impl ModuleResolver for ChainResolver {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        for resolver in &self.resolvers {
            if let Some(module) = resolver.resolve(path, base) {
                return Some(module);
            }
        }
        None
    }
}
