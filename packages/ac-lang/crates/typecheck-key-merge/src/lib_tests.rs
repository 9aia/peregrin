use super::*;
use ac_typecheck_core::{Type, TypeEnv};
use serde_json::json;

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
            _ => Type::Unknown}
    }
}

#[test]
fn non_merge_field_is_ok() {
    let interp = JsonInterpreter;
    let env = TypeEnv::new();
    let schema = json!({ "type": "string" });
    assert!(check_field("name", &schema, &interp, &env).is_ok());
}

#[test]
fn merge_object_is_ok() {
    let interp = JsonInterpreter;
    let env = TypeEnv::new();
    let schema = json!({
        "type": "object",
        "merge": true,
        "properties": {
            "host": { "type": "string" }
        }
    });
    assert!(check_field("_base", &schema, &interp, &env).is_ok());
}

#[test]
fn merge_non_object_is_error() {
    let interp = JsonInterpreter;
    let env = TypeEnv::new();
    let schema = json!({ "type": "string", "merge": true });
    let result = check_field("_base", &schema, &interp, &env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("object"));
}

#[test]
fn merge_compatible_with_parent() {
    let interp = JsonInterpreter;
    let mut env = TypeEnv::new();
    env.insert("host", Type::String);

    let schema = json!({
        "type": "object",
        "merge": true,
        "properties": {
            "host": { "type": "string" }
        }
    });
    assert!(check_field("_base", &schema, &interp, &env).is_ok());
}

#[test]
fn merge_incompatible_with_parent() {
    let interp = JsonInterpreter;
    let mut env = TypeEnv::new();
    env.insert("host", Type::String);

    let schema = json!({
        "type": "object",
        "merge": true,
        "properties": {
            "host": { "type": "number" }
        }
    });
    let result = check_field("_base", &schema, &interp, &env);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.field, "host");
}

#[test]
fn check_multiple_merge_fields() {
    let interp = JsonInterpreter;
    let env = TypeEnv::new();
    let schemas = vec![
        (
            "ok".to_string(),
            json!({ "type": "object", "merge": true, "properties": {} }),
        ),
        (
            "bad".to_string(),
            json!({ "type": "string", "merge": true }),
        ),
    ];
    let errors = check(&schemas, &interp, &env);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "bad");
}
