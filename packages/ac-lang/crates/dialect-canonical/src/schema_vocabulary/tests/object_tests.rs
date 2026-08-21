use super::*;
use ac_dialect::Schema;
use ac_runtime::NonFunctionValue;

// ── object keywords ────────────────────────────────────────────

#[test]
fn validate_required_fields() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "required": ["name", "age"]
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": "Luis", "age": 30})
            ))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"name": "Luis"})))
            .is_err()
    );
}

#[test]
fn validate_properties() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": "Luis", "age": 30})
            ))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": "Luis", "age": -1})
            ))
            .is_err()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": 42, "age": 30})
            ))
            .is_err()
    );
}

#[test]
fn validate_pattern_properties() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "patternProperties": {
            "^x-": {"type": "string"}
        }
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"x-custom": "hello"})))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"x-custom": 42})))
            .is_err()
    );
}

#[test]
fn validate_additional_properties_false() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "additionalProperties": false
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"name": "Luis"})))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": "Luis", "extra": 1})
            ))
            .is_err()
    );
}

#[test]
fn validate_additional_properties_schema() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "additionalProperties": {"type": "number"}
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": "Luis", "extra": 1})
            ))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"name": "Luis", "extra": "bad"})
            ))
            .is_err()
    );
}

#[test]
fn validate_min_max_properties() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "minProperties": 1,
        "maxProperties": 2
    }));
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({"a": 1})))
            .is_ok()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(serde_json::json!({})))
            .is_err()
    );
    assert!(
        schema
            .validate(&NonFunctionValue::new(
                serde_json::json!({"a": 1, "b": 2, "c": 3})
            ))
            .is_err()
    );
}
