use super::*;
use ac_dialect::Schema;
use ac_runtime::RuntimeValue;

// ── type ───────────────────────────────────────────────────────

#[test]
fn validate_string() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string"}));
    assert!(schema.validate(&RuntimeValue::new("hello")).is_ok());
    assert!(schema.validate(&RuntimeValue::new(42)).is_err());
}

#[test]
fn validate_integer() {
    let schema = JsonSchema::new(serde_json::json!({"type": "integer"}));
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(3.14)).is_err());
}

#[test]
fn validate_multi_type() {
    let schema = JsonSchema::new(serde_json::json!({"type": ["string", "null"]}));
    assert!(schema.validate(&RuntimeValue::new("hello")).is_ok());
    assert!(
        schema
            .validate(&RuntimeValue::new(serde_json::Value::Null))
            .is_ok()
    );
    assert!(schema.validate(&RuntimeValue::new(42)).is_err());
}

// ── enum / const ───────────────────────────────────────────────

#[test]
fn validate_enum() {
    let schema = JsonSchema::new(serde_json::json!({"enum": ["red", "green", "blue"]}));
    assert!(schema.validate(&RuntimeValue::new("red")).is_ok());
    assert!(schema.validate(&RuntimeValue::new("purple")).is_err());
}

#[test]
fn validate_const() {
    let schema = JsonSchema::new(serde_json::json!({"const": 42}));
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(43)).is_err());
}
