use serde_json::json;

use ac_typegen_core::TypegenContext;

use crate::expr;
use ac_dialect::dialect::dialect_stub::stub_vocab;

#[test]
fn get_expr_returns_string() {
    let schema = json!({ "type": "string", "expr": "compute()" });
    assert_eq!(
        expr::get_expr(&schema, stub_vocab()),
        Some("compute()".to_string())
    );
}

#[test]
fn get_expr_returns_none_when_absent() {
    let schema = json!({ "type": "string" });
    assert_eq!(expr::get_expr(&schema, stub_vocab()), None);
}

#[test]
fn strip_removes_expr_key() {
    let schemas = vec![(
        "host".to_string(),
        json!({ "type": "string", "expr": "compute()" }),
    )];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    expr::strip_key(&mut ctx, stub_vocab());
    let schema = &ctx.fields["host"];
    assert!(!schema.as_object().unwrap().contains_key("expr"));
    assert_eq!(schema["type"], "string");
}