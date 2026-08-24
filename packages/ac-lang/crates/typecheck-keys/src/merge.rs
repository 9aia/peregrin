//! # merge
//!
//! Typechecks the `merge` AC key.
//!
//! Fields with `merge: true` are objects whose properties are spread
//! into the parent scope at runtime. This module checks that:
//!
//! - Merge fields have type `object` (merging a non-object is a type error).
//! - Merged field types are compatible with the parent's declared types
//!   for the same field names.
//!
//! ## Architectural rule
//!
//! > **Merge fields must be objects, and their properties must be
//! > type-compatible with the parent scope.**

use ac_typecheck_core::{SchemaInterpreter, Type, TypeError, is_compatible};
use serde_json::Value as JsonValue;

/// The AC key this module handles.
pub const KEY: &str = "merge";

/// Check if a field schema has `merge: true`.
pub fn is_merge_field(schema: &JsonValue) -> bool {
    schema.get(KEY).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Check a single merge field.
///
/// Returns a [`TypeError`] if:
/// - The field is marked `merge: true` but is not declared as `object`.
/// - The field is an object but its properties conflict with parent types.
///
/// `parent_env` provides the parent scope's types for compatibility checks.
#[allow(clippy::result_large_err)]
pub fn check_field(
    field_name: &str,
    schema: &JsonValue,
    interpreter: &dyn SchemaInterpreter,
    parent_env: &ac_typecheck_core::TypeEnv,
) -> Result<(), TypeError> {
    if !is_merge_field(schema) {
        return Ok(());
    }

    let field_type = interpreter.interpret(schema);

    let merge_props = match &field_type {
        Type::Object(props) => props,
        _ => {
            return Err(TypeError::custom(
                field_name,
                format!(
                    "merge field '{}' must be an object, got {}",
                    field_name,
                    field_type.name()
                ),
                "",
            ));
        }
    };

    for (prop_name, prop_type) in merge_props {
        if let Some(parent_type) = parent_env.lookup(prop_name)
            && !is_compatible(prop_type, parent_type)
        {
            return Err(TypeError::mismatch(
                prop_name,
                parent_type.clone(),
                prop_type.clone(),
                field_name,
            ));
        }
    }

    Ok(())
}

/// Check all merge fields.
///
/// Returns a list of errors for all incompatible merge fields.
pub fn check(
    schemas: &[(String, JsonValue)],
    interpreter: &dyn SchemaInterpreter,
    parent_env: &ac_typecheck_core::TypeEnv,
) -> Vec<TypeError> {
    let mut errors = Vec::new();
    for (name, schema) in schemas {
        if let Err(e) = check_field(name, schema, interpreter, parent_env) {
            errors.push(e);
        }
    }
    errors
}

#[cfg(test)]
mod lib_tests;
