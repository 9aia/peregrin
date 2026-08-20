#![no_main]

use ac_fields::{eval_expression, NoFunctions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: String| {
    let mut ns = ac_fields::Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"name": "test", "value": 42}));
    ns.insert("env".to_string(), serde_json::json!({"HOME": "/tmp"}));

    let _ = eval_expression(&data, &ns, &NoFunctions);
});
