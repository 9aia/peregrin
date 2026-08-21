use super::*;
use ac_dialect::Schema;
use ac_runtime::RuntimeValue;

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
            .validate(&RuntimeValue::new(serde_json::json!({"name": "Luis"})))
            .is_ok()
    );
    assert!(
        schema
            .validate(&RuntimeValue::new(serde_json::json!({"age": 30})))
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
    assert!(schema.validate(&RuntimeValue::new("hello")).is_ok());
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(true)).is_err());
}

#[test]
fn validate_one_of() {
    let schema = JsonSchema::new(serde_json::json!({
        "oneOf": [
            {"type": "string"},
            {"type": "number"}
        ]
    }));
    assert!(schema.validate(&RuntimeValue::new("hello")).is_ok());
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(true)).is_err());
}

#[test]
fn validate_not() {
    let schema = JsonSchema::new(serde_json::json!({
        "not": {"type": "string"}
    }));
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
    assert!(schema.validate(&RuntimeValue::new("hello")).is_err());
}

// ── conditional keywords ───────────────────────────────────────

#[test]
fn validate_if_then_else() {
    let schema = JsonSchema::new(serde_json::json!({
        "if": {"type": "string"},
        "then": {"minLength": 3},
        "else": {"type": "number"}
    }));
    assert!(schema.validate(&RuntimeValue::new("abc")).is_ok());
    assert!(schema.validate(&RuntimeValue::new("ab")).is_err());
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(true)).is_err());
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
    assert!(schema.validate(&RuntimeValue::new(5)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(0)).is_err());
    assert!(schema.validate(&RuntimeValue::new("hello")).is_err());
}

// ── boolean schemas ────────────────────────────────────────────

#[test]
fn validate_boolean_true() {
    let schema = JsonSchema::new(serde_json::json!(true));
    assert!(schema.validate(&RuntimeValue::new("anything")).is_ok());
    assert!(schema.validate(&RuntimeValue::new(42)).is_ok());
}

#[test]
fn validate_boolean_false() {
    let schema = JsonSchema::new(serde_json::json!(false));
    assert!(schema.validate(&RuntimeValue::new("anything")).is_err());
}
