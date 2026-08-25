use handl_typecheck_core::{SchemaInterpreter, Type, TypeError, is_compatible};
use serde_json::Value as JsonValue;

/// Recursively check that `field` overrides in nested objects are
/// type-compatible with `root_type`.
pub fn check_in_children(
    value: &JsonValue,
    field: &str,
    root_type: &Type,
    _root_val: &JsonValue,
    interpreter: &dyn SchemaInterpreter,
    errors: &mut Vec<TypeError>,
    path: &str,
) {
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            let child_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };

            if key == field {
                let override_type = infer_value_type(val, interpreter);
                if !override_type.is_unknown() && !is_compatible(&override_type, root_type) {
                    errors.push(TypeError::mismatch(
                        field,
                        root_type.clone(),
                        override_type,
                        &child_path,
                    ));
                }
            }

            if val.is_object() {
                check_in_children(
                    val,
                    field,
                    root_type,
                    _root_val,
                    interpreter,
                    errors,
                    &child_path,
                );
            }
        }
    }
}

/// Infer the type of a concrete JSON value.
pub fn infer_value_type(value: &JsonValue, interpreter: &dyn SchemaInterpreter) -> Type {
    match value {
        JsonValue::Null => Type::Null,
        JsonValue::Bool(_) => Type::Boolean,
        JsonValue::Number(_) => Type::Number,
        JsonValue::String(_) => Type::String,
        JsonValue::Array(arr) => {
            let item_type = arr
                .first()
                .map(|v| infer_value_type(v, interpreter))
                .unwrap_or(Type::Unknown);
            Type::Array(Box::new(item_type))
        }
        JsonValue::Object(_) => {
            let fake_schema = serde_json::json!({ "type": "object" });
            interpreter.interpret(&fake_schema)
        }
    }
}
