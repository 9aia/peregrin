use super::*;
use handl_dialect::Schema;
use handl_vm_values_json::NonFunctionValue;

// ── number keywords ────────────────────────────────────────────

#[test]
fn validate_minimum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "minimum": 10}));
    assert!(schema.validate(&NonFunctionValue::new(10)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(9)).is_err());
}

#[test]
fn validate_maximum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "maximum": 100}));
    assert!(schema.validate(&NonFunctionValue::new(100)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(101)).is_err());
}

#[test]
fn validate_exclusive_minimum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "exclusiveMinimum": 0}));
    assert!(schema.validate(&NonFunctionValue::new(1)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(0)).is_err());
}

#[test]
fn validate_exclusive_maximum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "exclusiveMaximum": 100}));
    assert!(schema.validate(&NonFunctionValue::new(99)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(100)).is_err());
}

#[test]
fn validate_multiple_of() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "multipleOf": 3}));
    assert!(schema.validate(&NonFunctionValue::new(9)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(10)).is_err());
}
