use super::*;
use ac_typecheck_core::{SchemaInterpreter, Type};
use serde_json::{Value as JsonValue, json};

struct JsonInterpreter;

impl SchemaInterpreter for JsonInterpreter {
    fn interpret(&self, schema: &JsonValue) -> Type {
        match schema.get("type").and_then(|v| v.as_str()) {
            Some("string") => Type::String,
            Some("number") | Some("integer") => Type::Number,
            Some("boolean") => Type::Boolean,
            Some("object") => {
                let mut props = std::collections::BTreeMap::new();
                if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                    for (key, val) in properties {
                        props.insert(key.clone(), self.interpret(val));
                    }
                }
                Type::Object(props)
            }
            _ => Type::Unknown,
        }
    }
}

#[test]
fn no_inheritable_fields_no_errors() {
    let interp = JsonInterpreter;
    let data = json!({ "name": "alice" });
    let errors = check(&[], &data, &interp);
    assert!(errors.is_empty());
}

#[test]
fn compatible_override_no_errors() {
    let interp = JsonInterpreter;
    let schemas = vec![(
        "mode".to_string(),
        json!({ "type": "string", "inheritable": true }),
    )];
    let data = json!({
        "mode": "read-only",
        "config": {
            "mode": "read-write"
        }
    });
    let errors = check(&schemas, &data, &interp);
    assert!(errors.is_empty());
}

#[test]
fn incompatible_override_reports_error() {
    let interp = JsonInterpreter;
    let schemas = vec![(
        "mode".to_string(),
        json!({ "type": "string", "inheritable": true }),
    )];
    let data = json!({
        "mode": "read-only",
        "config": {
            "mode": 42
        }
    });
    let errors = check(&schemas, &data, &interp);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "mode");
    assert!(errors[0].path.contains("config"));
}

#[test]
fn deep_override_checked() {
    let interp = JsonInterpreter;
    let schemas = vec![(
        "mode".to_string(),
        json!({ "type": "string", "inheritable": true }),
    )];
    let data = json!({
        "mode": "read-only",
        "server": {
            "deep": {
                "mode": true
            }
        }
    });
    let errors = check(&schemas, &data, &interp);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].path.contains("server"));
    assert!(errors[0].path.contains("deep"));
}

#[test]
fn non_inheritable_field_not_checked() {
    let interp = JsonInterpreter;
    let schemas: Vec<(String, JsonValue)> = vec![];
    let data = json!({
        "name": "alice",
        "config": {
            "name": 42
        }
    });
    let errors = check(&schemas, &data, &interp);
    assert!(errors.is_empty());
}
