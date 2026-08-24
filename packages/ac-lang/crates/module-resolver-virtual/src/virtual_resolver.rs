use ac_runtime_values::NonFunctionValue;
use std::collections::HashMap;
use std::path::Path;

use serde_json::Value as JsonValue;

use ac_dialect_canonical::AC_DIALECT;
use ac_fields::extract_fields;
use ac_module_resolver_core::{ModuleResolver, ResolvedModule};

/// In-memory module resolver.
///
/// Maps module paths to pre-parsed JSON document data.
/// No filesystem I/O — all modules live in memory.
pub struct VirtualResolver {
    modules: HashMap<String, JsonValue>,
}

impl VirtualResolver {
    /// Create an empty resolver.
    pub fn new() -> Self {
        VirtualResolver {
            modules: HashMap::new(),
        }
    }

    /// Register a module by path with its parsed document data.
    pub fn add(&mut self, path: &str, data: JsonValue) {
        self.modules.insert(path.to_string(), data);
    }

    /// Register a module by path with raw content (parsed as YAML).
    pub fn add_raw(&mut self, path: &str, content: &str) {
        if let Ok(value) = ac_frontend_yaml::parse(content) {
            self.modules.insert(path.to_string(), value.inner().clone());
        }
    }
}

impl Default for VirtualResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver for VirtualResolver {
    fn resolve(&self, path: &str, _base: Option<&Path>) -> Option<ResolvedModule> {
        let data = self.modules.get(path)?;
        let value = NonFunctionValue::new(data.clone());
        let extracted = extract_fields(value, &AC_DIALECT).ok()?;

        Some(ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: None,
        })
    }
}
