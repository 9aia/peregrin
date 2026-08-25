//! # mutable
//!
//! Typechecks the `mutable` handl key.
//!
//! Fields with `mutable: false` cannot be overridden in nested scopes.
//! This module walks the data tree and reports any nested overrides of
//! immutable fields as [`TypeError`]s.
//!
//! ## Architectural rule
//!
//! > **Immutable fields cannot be overridden in nested scopes.**

use handl_typecheck_core::TypeError;
use serde_json::Value as JsonValue;

/// The handl key this module handles.
pub const KEY: &str = "mutable";

/// Check if a field schema has `mutable: false`.
pub fn is_immutable(schema: &JsonValue) -> bool {
    schema
        .get(KEY)
        .and_then(|v| v.as_bool())
        .map(|b| !b)
        .unwrap_or(false)
}

/// Check that fields declared `mutable: false` are not overridden in
/// nested scopes.
///
/// Walks the data tree. For each immutable field, compares every nested
/// occurrence against the root value. Returns [`TypeError`]s for any
/// mismatches.
///
/// - `immutable_fields` — field names declared `mutable: false`.
/// - `data` — the document data (after field extraction).
pub fn check(immutable_fields: &[String], data: &JsonValue) -> Vec<TypeError> {
    if immutable_fields.is_empty() {
        return Vec::new();
    }

    let mut errors = Vec::new();

    if let Some(obj) = data.as_object() {
        for field in immutable_fields {
            if let Some(root_val) = obj.get(field) {
                check_in_children(data, field, root_val, &mut errors, "");
            }
        }
    }

    errors
}

/// Recursively check that `field` is not overridden in nested objects.
fn check_in_children(
    value: &JsonValue,
    field: &str,
    root_val: &JsonValue,
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

            if key == field && val != root_val {
                errors.push(TypeError::custom(
                    field,
                    format!(
                        "'{field}' is immutable (mutable: false) \
                         — cannot override value from root scope"
                    ),
                    &child_path,
                ));
            }

            if val.is_object() {
                check_in_children(val, field, root_val, errors, &child_path);
            }
        }
    }
}

#[cfg(test)]
mod lib_tests;
