use std::path::Path;

use crate::module_resolver::ModuleResolver;
use crate::resolved_module::ResolvedModule;

/// A resolver that always returns `None`.
pub struct NoneResolver;

impl ModuleResolver for NoneResolver {
    fn resolve(&self, _path: &str, _base: Option<&Path>) -> Option<ResolvedModule> {
        None
    }
}
