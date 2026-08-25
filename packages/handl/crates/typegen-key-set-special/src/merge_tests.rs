use serde_json::json;

use handl_typegen_core::TypegenContext;

use crate::merge;
use handl_dialect::dialect::dialect_stub::stub_vocab;

#[test]
fn detects_merge_field() {
    assert!(merge::is_merge_field(
        &json!({ "type": "object", "merge": true }),
        stub_vocab()
    ));
    assert!(!merge::is_merge_field(
        &json!({ "type": "object" }),
        stub_vocab()
    ));
    assert!(!merge::is_merge_field(
        &json!({ "type": "object", "merge": false }),
        stub_vocab()
    ));
}

#[test]
fn filter_removes_named_fields() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        (
            "_base".to_string(),
            json!({ "type": "object", "merge": true }),
        ),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    merge::filter_fields(&mut ctx, &["_base".to_string()]);
    assert_eq!(ctx.fields.len(), 1);
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "host"));
}

#[test]
fn strip_removes_merge_key() {
    let schemas = vec![(
        "host".to_string(),
        json!({ "type": "string", "merge": false }),
    )];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    merge::strip_key(&mut ctx, stub_vocab());
    let schema = &ctx.fields["host"];
    assert!(!schema.as_object().unwrap().contains_key("merge"));
    assert_eq!(schema["type"], "string");
}
