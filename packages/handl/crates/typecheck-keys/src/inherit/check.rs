use std::collections::BTreeMap;

use handl_typecheck_core::{SchemaInterpreter, Type, TypeError};
use serde_json::Value as JsonValue;

use super::walker::check_in_children;

/// The handl key this module handles.
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
                    root_val,
                    interpreter,
                    &mut errors,
                    "",
                );
            }
        }
    }

    errors
}
