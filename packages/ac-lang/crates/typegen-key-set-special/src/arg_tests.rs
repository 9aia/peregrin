use serde_json::json;

use ac_typegen_core::TypegenContext;

use ac_dialect::dialect::dialect_stub::stub_vocab;
use crate::arg;

#[test]
fn has_arg_detects_key() {
    assert!(arg::has_arg(&json!({ "type": "string", "arg": true }), stub_vocab()));
    assert!(arg::has_arg(&json!({ "type": "string", "arg": { "type": "string" } }), stub_vocab()));
    assert!(!arg::has_arg(&json!({ "type": "string" }), stub_vocab()));
}

#[test]
fn strip_removes_arg_key() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string", "arg": true })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    arg::strip_key(&mut ctx, stub_vocab());
    let schema = &ctx.fields["host"];
    assert!(!schema.as_object().unwrap().contains_key("arg"));
    assert_eq!(schema["type"], "string");
}
