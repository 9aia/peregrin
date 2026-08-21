use super::*;
use ac_dialect::Schema;
use ac_runtime::RuntimeValue;

// ── array keywords ─────────────────────────────────────────────

#[test]
fn validate_array_items() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "array",
        "items": {"type": "string"}
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!(["a", "b", "c"]))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!(["a", 1]))).is_err());
}

#[test]
fn validate_prefix_items() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "array",
        "prefixItems": [{"type": "string"}, {"type": "number"}]
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!(["a", 1]))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, "a"]))).is_err());
}

#[test]
fn validate_min_items() {
    let schema = JsonSchema::new(serde_json::json!({"type": "array", "minItems": 2}));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, 2]))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1]))).is_err());
}

#[test]
fn validate_max_items() {
    let schema = JsonSchema::new(serde_json::json!({"type": "array", "maxItems": 3}));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, 2, 3]))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, 2, 3, 4]))).is_err());
}

#[test]
fn validate_unique_items() {
    let schema = JsonSchema::new(serde_json::json!({"type": "array", "uniqueItems": true}));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, 2, 3]))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, 2, 1]))).is_err());
}

#[test]
fn validate_contains() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "array",
        "contains": {"type": "string"}
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, "a", 2]))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!([1, 2, 3]))).is_err());
}
