use super::*;
use handl_dialect::Schema;
use handl_vm_values_json::NonFunctionValue;

// ── combinatorial keywords ─────────────────────────────────────

#[test]
fn validate_all_of() {
    let schema = JsonSchema::new(serde_json::json!({
        "allOf": [
            {"type": "object"},
            {"required": ["name"]}
        ]
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"name": "Luis"})))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"age": 30})))
            .is_err()
    );
}

#[test]
fn validate_any_of() {
    let schema = JsonSchema::new(serde_json::json!({
        "anyOf": [
            {"type": "string"},
            {"type": "number"}
        ]
    }));
    assert!(schema.validate(&NonFunctionValue::new("hello")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(42)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(true)).is_err());
}

#[test]
fn validate_one_of() {
    let schema = JsonSchema::new(serde_json::json!({
        "oneOf": [
            {"type": "string"},
            {"type": "number"}
        ]
    }));
    assert!(schema.validate(&NonFunctionValue::new("hello")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(42)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(true)).is_err());
}

#[test]
fn validate_not() {
    let schema = JsonSchema::new(serde_json::json!({
        "not": {"type": "string"}
    }));
    assert!(schema.validate(&NonFunctionValue::new(42)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new("hello")).is_err());
}

// ── conditional keywords ───────────────────────────────────────

#[test]
fn validate_if_then_else() {
    let schema = JsonSchema::new(serde_json::json!({
        "if": {"type": "string"},
        "then": {"minLength": 3},
        "else": {"type": "number"}
    }));
    assert!(schema.validate(&NonFunctionValue::new("abc")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new("ab")).is_err());
    assert!(schema.validate(&NonFunctionValue::new(42)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(true)).is_err());
}

// ── $ref ───────────────────────────────────────────────────────

#[test]
fn validate_ref() {
    let schema = JsonSchema::new(serde_json::json!({
        "$defs": {
            "positiveInt": {"type": "integer", "minimum": 1}
        },
        "$ref": "#/$defs/positiveInt"
    }));
    assert!(schema.validate(&NonFunctionValue::new(5)).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(0)).is_err());
    assert!(schema.validate(&NonFunctionValue::new("hello")).is_err());
}

// ── boolean schemas ────────────────────────────────────────────

#[test]
fn validate_boolean_true() {
    let schema = JsonSchema::new(serde_json::json!(true));
    assert!(schema.validate(&NonFunctionValue::new("anything")).is_ok());
    assert!(schema.validate(&NonFunctionValue::new(42)).is_ok());
}

#[test]
fn validate_boolean_false() {
    let schema = JsonSchema::new(serde_json::json!(false));
    assert!(schema.validate(&NonFunctionValue::new("anything")).is_err());
}
