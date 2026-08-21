use crate::resolved_module::ResolvedModule;

use serde_json::json;

pub fn make_resolved_module(name: &str) -> ResolvedModule {
    ResolvedModule {
        schemas: vec![(name.to_string(), json!({"type": "object"}))],
        merge: vec![],
        functions: vec![],
        data: json!(null),
        base_path: None,
    }
}
