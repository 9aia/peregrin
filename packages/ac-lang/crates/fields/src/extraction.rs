use ac_runtime::RuntimeValue;
use serde_json::Value as JsonValue;

use crate::types::{
    ArgSpec, ComputedField, ExtractedFields, FunctionSignature, RESERVED_FIELDS,
};

/// Extracts the `fields` and `functions` keys from a value, returning
/// `(field_schemas, function_signatures, data_without_fields_or_functions)`.
///
/// Each entry in `field_schemas` is `(name, raw_schema_value)`.
/// Each entry in `function_signatures` is `(name, FunctionSignature)`.
/// The caller is responsible for constructing `Schema<V>`
/// implementations from the raw values.
///
/// Returns an error if a declared field name is reserved.
pub fn extract_fields(
    value: RuntimeValue,
) -> Result<ExtractedFields, String> {
    let mut schemas = Vec::new();
    let mut functions = Vec::new();
    let mut data = value.clone();

    if let Some(obj) = value.as_object() {
        if let Some(fields) = obj.get("fields").and_then(|v| v.as_object()) {
            for (name, schema_val) in fields {
                if RESERVED_FIELDS.contains(&name.as_str()) {
                    return Err(format!(
                        "'{name}' is a reserved field name and cannot be declared in fields:"
                    ));
                }
                schemas.push((name.clone(), schema_val.clone()));
            }
            if let Some(data_obj) = data.as_object_mut() {
                data_obj.remove("fields");
            }
        }

        if let Some(fns) = obj.get("functions").and_then(|v| v.as_object()) {
            for (name, sig_val) in fns {
                let sig = parse_function_signature(sig_val)?;
                functions.push((name.clone(), sig));
            }
            if let Some(data_obj) = data.as_object_mut() {
                data_obj.remove("functions");
            }
        }
    }

    // Extract inheritable and immutable field names from schemas
    let inheritable: Vec<String> = schemas
        .iter()
        .filter(|(_, raw)| raw.get("inheritable").and_then(|v| v.as_bool()) == Some(true))
        .map(|(name, _)| name.clone())
        .collect();
    let immutable: Vec<String> = schemas
        .iter()
        .filter(|(_, raw)| raw.get("mutable").and_then(|v| v.as_bool()) == Some(false))
        .map(|(name, _)| name.clone())
        .collect();

    // Extract computed field expressions from schemas
    let computed: Vec<ComputedField> = schemas
        .iter()
        .filter_map(|(name, raw)| {
            raw.get("expr")
                .and_then(|v| v.as_str())
                .map(|s| {
                    let arg = raw.get("arg").and_then(|v| {
                        if v.is_boolean() && v.as_bool() == Some(true) {
                            Some(ArgSpec::Untyped)
                        } else if v.is_object() {
                            Some(ArgSpec::Typed(v.clone()))
                        } else {
                            None
                        }
                    });
                    ComputedField {
                        name: name.clone(),
                        expr: s.to_string(),
                        arg,
                    }
                })
        })
        .collect();

    // Extract merge field names from schemas
    let merge: Vec<String> = schemas
        .iter()
        .filter(|(_, raw)| raw.get("merge").and_then(|v| v.as_bool()) == Some(true))
        .map(|(name, _)| name.clone())
        .collect();

    Ok(ExtractedFields { schemas, functions, data, inheritable, immutable, computed, merge })
}

/// Parses a function signature from a raw JSON value.
///
/// Expected format:
/// ```json
/// {
///   "params": { "x": { "type": "number" }, "y": { "type": "string" } },
///   "return": { "type": "number" }
/// }
/// ```
fn parse_function_signature(val: &JsonValue) -> Result<FunctionSignature, String> {
    let mut params = Vec::new();
    if let Some(p) = val.get("params").and_then(|v| v.as_object()) {
        for (name, schema) in p {
            params.push((name.clone(), schema.clone()));
        }
    }
    let return_schema = val.get("return").cloned();
    let body = val.get("body").and_then(|v| v.as_str()).map(|s| s.to_string());
    Ok(FunctionSignature { params, return_schema, body })
}

/// Returns the names of fields present in `data` but not declared in
/// `declared_names`.
pub fn undeclared_fields<'a>(
    data: &'a RuntimeValue,
    declared: &[&str],
) -> Vec<&'a str> {
    let declared_set: std::collections::HashSet<&str> = declared.iter().copied().collect();
    data.as_object()
        .map(|obj| {
            obj.keys()
                .filter(|k| !declared_set.contains(k.as_str()))
                .map(|k| k.as_str())
                .collect()
        })
        .unwrap_or_default()
}
