#![no_main]

use handl_fields::{interpolate, interpolate_scoped, Namespaces, NoFunctions};
use libfuzzer_sys::fuzz_target;
use serde_json::Value as JsonValue;

fuzz_target!(|data: String| {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"name": "test", "value": 42}));
    ns.insert("env".to_string(), serde_json::json!({"HOME": "/tmp"}));

    let value = JsonValue::String(data);
    let _ = interpolate(&value, &ns, &NoFunctions, &handl_dialect_canonical::AC_DIALECT);
});
