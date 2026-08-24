use std::path::Path;

use crate::module_resolver::ModuleResolver;
use crate::resolved_module::ResolvedModule;

/// A resolver that returns a static value.
pub struct StaticResolver(pub Option<ResolvedModule>);

impl ModuleResolver for StaticResolver {
    fn resolve(&self, _path: &str, _base: Option<&Path>) -> Option<ResolvedModule> {
        self.0.clone()
    }
}
