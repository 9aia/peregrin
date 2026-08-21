use super::*;
use ac_dialect::Schema;
use ac_runtime::NonFunctionValue;

// ── string keywords ────────────────────────────────────────────

#[test]
fn validate_min_length() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string", "minLength": 3}));
    assert!(schema.validate(&NonFunctionValue::new("abc")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new("ab")).is_err());
}

#[test]
fn validate_max_length() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string", "maxLength": 5}));
    assert!(schema.validate(&NonFunctionValue::new("hello")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new("toolong")).is_err());
}

#[test]
fn validate_pattern() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string", "pattern": "^[a-z]+$"}));
    assert!(schema.validate(&NonFunctionValue::new("abc")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new("ABC")).is_err());
}
