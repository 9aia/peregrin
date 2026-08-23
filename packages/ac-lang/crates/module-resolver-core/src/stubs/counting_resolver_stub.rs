use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::module_resolver::ModuleResolver;
use crate::resolved_module::ResolvedModule;
use crate::resolved_module_test_helpers::make_resolved_module;

/// A resolver that counts how many times `resolve` is called.
pub struct CountingResolver {
    pub count: Arc<AtomicU32>}

impl ModuleResolver for CountingResolver {
    fn resolve(&self, path: &str, _base: Option<&Path>) -> Option<ResolvedModule> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Some(make_resolved_module(path))
    }
}
