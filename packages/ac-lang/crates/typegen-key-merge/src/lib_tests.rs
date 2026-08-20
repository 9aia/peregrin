use serde_json::json;

use super::*;

#[test]
fn detects_merge_field() {
    assert!(is_merge_field(&json!({ "type": "object", "merge": true })));
    assert!(!is_merge_field(&json!({ "type": "object" })));
    assert!(!is_merge_field(&json!({ "type": "object", "merge": false })));
}

#[test]
fn filter_removes_named_fields() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        ("_base".to_string(), json!({ "type": "object", "merge": true })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    filter_fields(&mut ctx, &["_base".to_string()]);
    assert_eq!(ctx.fields.len(), 1);
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "host"));
}

#[test]
fn strip_removes_merge_key() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string", "merge": false })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    strip_key(&mut ctx);
    let schema = &ctx.fields["host"];
    assert!(!schema.as_object().unwrap().contains_key(KEY));
    assert_eq!(schema["type"], "string");
}
