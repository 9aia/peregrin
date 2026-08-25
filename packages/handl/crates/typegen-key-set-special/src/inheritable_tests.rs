use serde_json::json;

use handl_typegen_core::TypegenContext;

use crate::inheritable;
use handl_dialect::dialect::dialect_stub::stub_vocab;

#[test]
fn strip_removes_inheritable_key() {
    let schemas = vec![(
        "port".to_string(),
        json!({ "type": "number", "inheritable": true }),
    )];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    inheritable::strip_key(&mut ctx, stub_vocab());
    let schema = &ctx.fields["port"];
    assert!(!schema.as_object().unwrap().contains_key("inheritable"));
    assert_eq!(schema["type"], "number");
}
