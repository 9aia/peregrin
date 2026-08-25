use serde_json::json;

use handl_typegen_core::TypegenContext;

use crate::mutable;
use handl_dialect::dialect::dialect_stub::stub_vocab;

#[test]
fn strip_removes_mutable_key() {
    let schemas = vec![(
        "port".to_string(),
        json!({ "type": "number", "mutable": false }),
    )];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    mutable::strip_key(&mut ctx, stub_vocab());
    let schema = &ctx.fields["port"];
    assert!(!schema.as_object().unwrap().contains_key("mutable"));
    assert_eq!(schema["type"], "number");
}
