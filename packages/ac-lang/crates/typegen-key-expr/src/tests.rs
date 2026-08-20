use serde_json::json;

use super::*;

#[test]
fn get_expr_returns_string() {
    let schema = json!({ "type": "string", "expr": "compute()" });
    assert_eq!(get_expr(&schema), Some("compute()".to_string()));
}

#[test]
fn get_expr_returns_none_when_absent() {
    let schema = json!({ "type": "string" });
    assert_eq!(get_expr(&schema), None);
}

#[test]
fn strip_removes_expr_key() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string", "expr": "compute()" })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    strip_key(&mut ctx);
    let (_, schema) = &ctx.fields[0];
    assert!(!schema.as_object().unwrap().contains_key(KEY));
    assert_eq!(schema["type"], "string");
}
