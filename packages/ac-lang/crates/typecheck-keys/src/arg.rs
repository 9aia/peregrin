//! # arg
//!
//! Typechecks the `arg` AC key.
//!
//! The `arg` key declares a parameter type for a computed field — the
//! field's own data value is available as `${arg}` inside its `expr:`.
//! This module checks that:
//!
//! - The `arg` value is a valid schema fragment (has a `type` or is an
//!   object with schema-like structure).
//! - The `arg` type is compatible with the field's declared type (the
//!   arg is the field's own value, so it should match).
//!
//! ## Architectural rule
//!
//! > **The arg type must be compatible with the field's declared type.**

use ac_typecheck_core::{SchemaInterpreter, TypeError, is_compatible};
use serde_json::Value as JsonValue;

/// The AC key this module handles.
pub const KEY: &str = "arg";

/// Check if a field schema has an `arg` key.
pub fn has_arg(schema: &JsonValue) -> bool {
    schema.get(KEY).is_some()
}

/// Get the raw `arg` schema from a field schema, if present.
pub fn get_arg_schema(schema: &JsonValue) -> Option<&JsonValue> {
    schema.get(KEY)
}

/// Check a single field's `arg` against its declared type.
///
/// Returns a [`TypeError`] if the arg type is incompatible with the
/// declared type. Returns `Ok(())` if the arg is absent or compatible.
#[allow(clippy::result_large_err)]
pub fn check_field(
    field_name: &str,
    schema: &JsonValue,
    interpreter: &dyn SchemaInterpreter,
) -> Result<(), TypeError> {
    let Some(arg_schema) = get_arg_schema(schema) else {
        return Ok(());
    };

    let declared = interpreter.interpret(schema);
    let arg_type = interpreter.interpret(arg_schema);

    if arg_type.is_unknown() {
        return Ok(());
    }

    if !is_compatible(&arg_type, &declared) {
        return Err(TypeError::mismatch(field_name, declared, arg_type, ""));
    }

    Ok(())
}

/// Check all fields with `arg:` declarations.
///
/// Returns a list of errors for all incompatible arg types.
pub fn check(
    schemas: &[(String, JsonValue)],
    interpreter: &dyn SchemaInterpreter,
) -> Vec<TypeError> {
    let mut errors = Vec::new();
    for (name, schema) in schemas {
        if let Err(e) = check_field(name, schema, interpreter) {
            errors.push(e);
        }
    }
    errors
}

#[cfg(test)]
mod lib_tests;
