use std::collections::BTreeMap;

use ac_typecheck_core::{FunctionType, Type, TypeParam, Variance};
use serde_json::Value as JsonValue;

/// Convert a JSON Schema fragment into a [`Type`].
///
/// In addition to standard JSON Schema `type` keywords, this interpreter
/// recognizes an AC-specific extension: `{ "type": "var", "name": "T" }`
/// produces `Type::Var("T")`, used in generic function signatures.
pub fn interpret_schema(schema: &JsonValue) -> Type {
    let Some(type_name) = schema.get("type").and_then(|v| v.as_str()) else {
        if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array())
            && let Some(first) = enum_vals.first()
        {
            return infer_json_type(first);
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
                .map(interpret_schema)
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
        "function" => {
            let type_params = schema
                .get("typeParams")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            if let Some(s) = v.as_str() {
                                return Some(TypeParam::new(s));
                            }
                            let obj = v.as_object()?;
                            let name = obj.get("name")?.as_str()?.to_string();
                            let bounds = obj
                                .get("bounds")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|b| {
                                            let btype = b.get("type")?.as_str()?;
                                            match btype {
                                                "trait" => b
                                                    .get("name")?
                                                    .as_str()
                                                    .map(|n| Type::Trait(n.to_string())),
                                                _ => None,
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            let variance = obj
                                .get("variance")
                                .and_then(|v| v.as_str())
                                .and_then(|s| match s {
                                    "covariant" | "out" => Some(Variance::Covariant),
                                    "contravariant" | "in" => Some(Variance::Contravariant),
                                    "invariant" => Some(Variance::Invariant),
                                    "bivariant" => Some(Variance::Bivariant),
                                    _ => None,
                                })
                                .unwrap_or(Variance::Invariant);
                            let tp = if bounds.is_empty() {
                                TypeParam::new(name)
                            } else {
                                TypeParam::with_bounds(name, bounds)
                            };
                            Some(tp.with_variance(variance))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let params = schema
                .get("params")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let name = p.get("name")?.as_str()?.to_string();
                            let param_schema = p.get("schema").unwrap_or(&JsonValue::Null);
                            Some((name, interpret_schema(param_schema)))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let return_type = schema
                .get("return")
                .map(interpret_schema)
                .unwrap_or(Type::Unknown);
            Type::Function(FunctionType {
                type_params,
                params,
                return_type: Box::new(return_type),
            })
        }
        "var" => schema
            .get("name")
            .and_then(|v| v.as_str())
            .map(|name| Type::Var(name.to_string()))
            .unwrap_or(Type::Unknown),
        "trait" => schema
            .get("name")
            .and_then(|v| v.as_str())
            .map(|name| Type::Trait(name.to_string()))
            .unwrap_or(Type::Unknown),
        "app" => {
            let ctor = schema
                .get("ctor")
                .map(interpret_schema)
                .unwrap_or(Type::Unknown);
            let args = schema
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().map(interpret_schema).collect::<Vec<_>>())
                .unwrap_or_default();
            Type::App(Box::new(ctor), args)
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
            let item = arr.first().map(infer_json_type).unwrap_or(Type::Unknown);
            Type::Array(Box::new(item))
        }
        JsonValue::Object(obj) => {
            // A function value is detected by having a `body` string field.
            if obj.get("body").is_some_and(|v| v.is_string()) {
                let params = obj
                    .get("params")
                    .map(|p| {
                        if let Some(arr) = p.as_array() {
                            arr.iter()
                                .filter_map(|p| {
                                    let name = p.get("name")?.as_str()?.to_string();
                                    let schema = p.get("schema").unwrap_or(&JsonValue::Null);
                                    Some((name, interpret_schema(schema)))
                                })
                                .collect::<Vec<_>>()
                        } else if let Some(map) = p.as_object() {
                            map.iter()
                                .map(|(name, schema)| (name.clone(), interpret_schema(schema)))
                                .collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        }
                    })
                    .unwrap_or_default();
                let return_type = obj
                    .get("return")
                    .map(interpret_schema)
                    .unwrap_or(Type::Unknown);
                Type::Function(FunctionType {
                    type_params: Vec::new(),
                    params,
                    return_type: Box::new(return_type),
                })
            } else {
                Type::Unknown
            }
        }
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
        type_params: Vec::new(),
        params: param_types,
        return_type,
    }
}