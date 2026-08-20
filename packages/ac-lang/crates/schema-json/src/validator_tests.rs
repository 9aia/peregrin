use super::*;
use ac_schema::Schema;
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
    assert!(schema.validate(&RuntimeValue::new(serde_json::Value::Null)).is_ok());
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

// ── string keywords ────────────────────────────────────────────

#[test]
fn validate_min_length() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string", "minLength": 3}));
    assert!(schema.validate(&RuntimeValue::new("abc")).is_ok());
    assert!(schema.validate(&RuntimeValue::new("ab")).is_err());
}

#[test]
fn validate_max_length() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string", "maxLength": 5}));
    assert!(schema.validate(&RuntimeValue::new("hello")).is_ok());
    assert!(schema.validate(&RuntimeValue::new("toolong")).is_err());
}

#[test]
fn validate_pattern() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string", "pattern": "^[a-z]+$"}));
    assert!(schema.validate(&RuntimeValue::new("abc")).is_ok());
    assert!(schema.validate(&RuntimeValue::new("ABC")).is_err());
}

// ── number keywords ────────────────────────────────────────────

#[test]
fn validate_minimum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "minimum": 10}));
    assert!(schema.validate(&RuntimeValue::new(10)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(9)).is_err());
}

#[test]
fn validate_maximum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "maximum": 100}));
    assert!(schema.validate(&RuntimeValue::new(100)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(101)).is_err());
}

#[test]
fn validate_exclusive_minimum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "exclusiveMinimum": 0}));
    assert!(schema.validate(&RuntimeValue::new(1)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(0)).is_err());
}

#[test]
fn validate_exclusive_maximum() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "exclusiveMaximum": 100}));
    assert!(schema.validate(&RuntimeValue::new(99)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(100)).is_err());
}

#[test]
fn validate_multiple_of() {
    let schema = JsonSchema::new(serde_json::json!({"type": "number", "multipleOf": 3}));
    assert!(schema.validate(&RuntimeValue::new(9)).is_ok());
    assert!(schema.validate(&RuntimeValue::new(10)).is_err());
}

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

// ── object keywords ────────────────────────────────────────────

#[test]
fn validate_required_fields() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "required": ["name", "age"]
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis", "age": 30}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis"}))).is_err());
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
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis", "age": 30}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis", "age": -1}))).is_err());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": 42, "age": 30}))).is_err());
}

#[test]
fn validate_pattern_properties() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "patternProperties": {
            "^x-": {"type": "string"}
        }
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"x-custom": "hello"}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"x-custom": 42}))).is_err());
}

#[test]
fn validate_additional_properties_false() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "additionalProperties": false
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis"}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis", "extra": 1}))).is_err());
}

#[test]
fn validate_additional_properties_schema() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "additionalProperties": {"type": "number"}
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis", "extra": 1}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis", "extra": "bad"}))).is_err());
}

#[test]
fn validate_min_max_properties() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "minProperties": 1,
        "maxProperties": 2
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"a": 1}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({}))).is_err());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"a": 1, "b": 2, "c": 3}))).is_err());
}

// ── combinatorial keywords ─────────────────────────────────────

#[test]
fn validate_all_of() {
    let schema = JsonSchema::new(serde_json::json!({
        "allOf": [
            {"type": "object"},
            {"required": ["name"]}
        ]
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"name": "Luis"}))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({"age": 30}))).is_err());
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

// ── nested / complex schemas ───────────────────────────────────

#[test]
fn validate_nested_object() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "uniqueItems": true
                    }
                },
                "required": ["name"]
            }
        },
        "required": ["user"]
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({
        "user": {"name": "Luis", "tags": ["dev", "rust"]}
    }))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({
        "user": {"tags": ["dev"]}
    }))).is_err());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({}))).is_err());
}

#[test]
fn validate_error_has_path() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    let result = schema.validate(&RuntimeValue::new(serde_json::json!({"age": -1})));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.path.contains("age"), "error path should contain 'age': got '{}'", err.path);
}

#[test]
fn validate_error_has_location_field() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    let result = schema.validate(&RuntimeValue::new(serde_json::json!({"age": -1})));
    let err = result.unwrap_err();
    assert!(err.location.is_none(), "location should be None from validator; engine resolves it post-hoc");
}

#[test]
fn validate_error_display_with_location() {
    use ac_source::SourceLocation;
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    let result = schema.validate(&RuntimeValue::new(serde_json::json!({"age": -1})));
    let mut err = result.unwrap_err();
    err.location = Some(SourceLocation::new(3, 5));
    let displayed = err.to_string();
    assert!(displayed.contains("[3:5]"), "display should contain [3:5]: got '{displayed}'");
    assert!(displayed.contains("age"), "display should contain path: got '{displayed}'");
}
