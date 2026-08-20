use std::collections::BTreeMap;

use ac_typecheck_core::{FunctionType, Type};
use serde_json::Value as JsonValue;

/// Convert a JSON Schema fragment into a [`Type`].
pub fn interpret_schema(schema: &JsonValue) -> Type {
    let Some(type_name) = schema.get("type").and_then(|v| v.as_str()) else {
        if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
            if let Some(first) = enum_vals.first() {
                return infer_json_type(first);
            }
        }
        if schema.is_object() {
            return Type::Unknown;
        }
        return infer_json_type(schema);
    };

    match type_name {
        "string" => Type::String,
        "number" | "integer" => Type::Number,
        "boolean" => Type::Boolean,
        "null" => Type::Null,
        "array" => {
            let item_type = schema
                .get("items")
                .map(|i| interpret_schema(i))
                .unwrap_or(Type::Unknown);
            Type::Array(Box::new(item_type))
        }
        "object" => {
            let mut props = BTreeMap::new();
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, val) in properties {
                    props.insert(key.clone(), interpret_schema(val));
                }
            }
            Type::Object(props)
        }
        _ => Type::Unknown,
    }
}

/// Infer the [`Type`] of a concrete JSON value.
pub fn infer_json_type(value: &JsonValue) -> Type {
    match value {
        JsonValue::Null => Type::Null,
        JsonValue::Bool(_) => Type::Boolean,
        JsonValue::Number(_) => Type::Number,
        JsonValue::String(_) => Type::String,
        JsonValue::Array(arr) => {
            let item = arr
                .first()
                .map(infer_json_type)
                .unwrap_or(Type::Unknown);
            Type::Array(Box::new(item))
        }
        JsonValue::Object(_) => Type::Unknown,
    }
}

/// Build a [`FunctionType`] from a raw function signature.
///
/// `params` is `(param_name, raw_schema)` pairs.
/// `return_schema` is the raw return type schema, if declared.
pub fn interpret_function(
    params: &[(String, JsonValue)],
    return_schema: Option<&JsonValue>,
) -> FunctionType {
    let param_types: Vec<(String, Type)> = params
        .iter()
        .map(|(name, schema)| (name.clone(), interpret_schema(schema)))
        .collect();

    let return_type = match return_schema {
        Some(schema) => Box::new(interpret_schema(schema)),
        None => Box::new(Type::Unknown),
    };

    FunctionType {
        params: param_types,
        return_type,
    }
}
