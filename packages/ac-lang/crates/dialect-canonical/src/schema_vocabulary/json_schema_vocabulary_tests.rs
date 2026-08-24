use super::*;
use ac_dialect::{Schema, SchemaVocabulary};
use ac_runtime_values::NonFunctionValue;

// ── id ─────────────────────────────────────────────────────────

#[test]
fn vocabulary_id() {
    assert_eq!(JSON_SCHEMA_VOCABULARY.id(), "json-schema-draft-2020-12");
}

// ── compile ────────────────────────────────────────────────────

#[test]
fn compile_provides_validating_schema() {
    let schema = JSON_SCHEMA_VOCABULARY
        .compile(&serde_json::json!({"type": "string"}))
        .unwrap();
    assert!(schema.validate(&NonFunctionValue::new("hello")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(42)).is_err());
}

#[test]
fn compile_boolean_true_schema() {
    let schema = JSON_SCHEMA_VOCABULARY
        .compile(&serde_json::json!(true))
        .unwrap();
    assert!(schema.validate(&NonFunctionValue::new("anything")).is_ok());
}

#[test]
fn compile_boolean_false_schema() {
    let schema = JSON_SCHEMA_VOCABULARY
        .compile(&serde_json::json!(false))
        .unwrap();
    assert!(schema.validate(&NonFunctionValue::new("anything")).is_err());
}
