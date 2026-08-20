//! # ac-typecheck-key-inherit
//!
//! Typechecks the `inheritable` AC key.
//!
//! Fields with `inheritable: true` propagate their values into nested
//! objects. This crate checks that override values in nested scopes have
//! types compatible with the parent declaration.
//!
//! ## Architectural rule
//!
//! > **Inherited field overrides must be type-compatible with the
//! > parent declaration.**

use std::collections::BTreeMap;

use ac_typecheck_core::{is_compatible, SchemaInterpreter, Type, TypeError};
use serde_json::Value as JsonValue;

/// The AC key this crate handles.
pub const KEY: &str = "inheritable";

/// Check if a field schema has `inheritable: true`.
pub fn is_inheritable(schema: &JsonValue) -> bool {
    schema.get(KEY).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Check that inherited field overrides in nested data are type-compatible.
///
/// Walks the data tree. For each inheritable field, compares every nested
/// occurrence's type against the root declared type. Returns errors for
/// any mismatches.
///
/// - `inheritable_schemas` — `(field_name, raw_schema)` pairs for fields
///   marked `inheritable: true`.
/// - `data` — the document data (after field extraction).
/// - `interpreter` — converts raw schemas to types.
pub fn check(
    inheritable_schemas: &[(String, JsonValue)],
    data: &JsonValue,
    interpreter: &dyn SchemaInterpreter,
) -> Vec<TypeError> {
    let mut errors = Vec::new();

    let root_types: BTreeMap<String, Type> = inheritable_schemas
        .iter()
        .map(|(name, schema)| (name.clone(), interpreter.interpret(schema)))
        .collect();

    if root_types.is_empty() {
        return errors;
    }

    if let Some(obj) = data.as_object() {
        for (field_name, root_type) in &root_types {
            if let Some(root_val) = obj.get(field_name) {
                check_in_children(
                    data,
                    field_name,
                    root_type,
                    &root_val,
                    interpreter,
                    &mut errors,
                    "",
                );
            }
        }
    }

    errors
}

/// Recursively check that `field` overrides in nested objects are
/// type-compatible with `root_type`.
fn check_in_children(
    value: &JsonValue,
    field: &str,
    root_type: &Type,
    root_val: &JsonValue,
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
                    root_val,
                    interpreter,
                    errors,
                    &child_path,
                );
            }
        }
    }
}

/// Infer the type of a concrete JSON value.
fn infer_value_type(value: &JsonValue, interpreter: &dyn SchemaInterpreter) -> Type {
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

#[cfg(test)]
mod lib_tests;
