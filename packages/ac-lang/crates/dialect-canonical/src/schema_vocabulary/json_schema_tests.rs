use super::*;
use ac_dialect::Schema;
use ac_runtime::RuntimeValue;

// ── parse ──────────────────────────────────────────────────────

#[test]
fn parse_valid_json() {
    let schema = JsonSchema::parse(r#"{"type": "string"}"#).unwrap();
    assert_eq!(schema.type_name(), Some("string"));
}

#[test]
fn parse_invalid_json() {
    assert!(JsonSchema::parse("not json").is_err());
}

// ── type_name ──────────────────────────────────────────────────

#[test]
fn type_name_single_type() {
    let schema = JsonSchema::new(serde_json::json!({"type": "integer"}));
    assert_eq!(schema.type_name(), Some("integer"));
}

#[test]
fn type_name_multi_type_returns_first() {
    let schema = JsonSchema::new(serde_json::json!({"type": ["string", "null"]}));
    assert_eq!(schema.type_name(), Some("string"));
}

#[test]
fn type_name_missing_type() {
    let schema = JsonSchema::new(serde_json::json!({"enum": [1, 2, 3]}));
    assert_eq!(schema.type_name(), None);
}

// ── new ────────────────────────────────────────────────────────

#[test]
fn new_stores_raw_value() {
    let raw = serde_json::json!({"type": "boolean"});
    let schema = JsonSchema::new(raw.clone());
    assert_eq!(schema.raw, raw);
}
