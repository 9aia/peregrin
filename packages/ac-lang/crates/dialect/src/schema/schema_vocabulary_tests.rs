use crate::dialect::Dialect;
use crate::dialect::dialect_stub::StubDialect;

#[test]
fn schema_vocabulary_id() {
    let d = StubDialect;
    assert_eq!(d.schema_vocabulary().id(), "stub");
}

#[test]
fn schema_vocabulary_compile_returns_validator() {
    let d = StubDialect;
    let raw = serde_json::json!({"type": "string"});
    let schema = d.schema_vocabulary().compile(&raw);
    assert!(schema.is_ok());
}
