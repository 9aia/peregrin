use serde_json::json;

use super::*;

#[test]
fn strip_removes_inheritable_key() {
    let schemas = vec![
        ("port".to_string(), json!({ "type": "number", "inheritable": true })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    strip_key(&mut ctx);
    let (_, schema) = &ctx.fields[0];
    assert!(!schema.as_object().unwrap().contains_key(KEY));
    assert_eq!(schema["type"], "number");
}
