use serde_json::json;

use super::*;

#[test]
fn has_arg_detects_key() {
    assert!(has_arg(&json!({ "type": "string", "arg": true })));
    assert!(has_arg(&json!({ "type": "string", "arg": { "type": "string" } })));
    assert!(!has_arg(&json!({ "type": "string" })));
}

#[test]
fn strip_removes_arg_key() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string", "arg": true })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    strip_key(&mut ctx);
    let (_, schema) = &ctx.fields[0];
    assert!(!schema.as_object().unwrap().contains_key(KEY));
    assert_eq!(schema["type"], "string");
}
