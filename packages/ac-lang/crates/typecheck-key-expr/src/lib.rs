//! # ac-typecheck-key-expr
//!
//! Typechecks the `expr` AC key.
//!
//! For fields with an `expr:` declaration, infers the expression's type
//! and checks it against the field's declared schema type.
//!
//! ## Architectural rule
//!
//! > **The inferred type must be compatible with the declared type.**

use ac_typecheck_core::{is_compatible, SchemaInterpreter, TypeEnv, TypeError};
use ac_typecheck_infer::{infer, FunctionTable};
use serde_json::Value as JsonValue;

/// The AC key this crate handles.
pub const KEY: &str = "expr";

/// Get the expression string from a field schema, if present.
pub fn get_expr(schema: &JsonValue) -> Option<String> {
    schema.get(KEY).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Check a single field's `expr` against its declared type.
///
/// Returns a [`TypeError`] if the inferred type is incompatible with
/// the declared type. Returns `Ok(())` if the expression is absent or
/// the types are compatible.
pub fn check_field(
    field_name: &str,
    schema: &JsonValue,
    env: &TypeEnv,
    functions: &FunctionTable,
    interpreter: &dyn SchemaInterpreter,
) -> Result<(), TypeError> {
    let Some(expr_str) = get_expr(schema) else {
        return Ok(());
    };

    let declared = interpreter.interpret(schema);
    let inferred = infer(&expr_str, env, functions);

    if inferred.is_unknown() {
        return Ok(());
    }

    if !is_compatible(&inferred, &declared) {
        return Err(TypeError::mismatch(
            field_name,
            declared,
            inferred,
            "",
        ));
    }

    Ok(())
}

/// Check all fields with `expr:` declarations.
///
/// `schemas` is `(field_name, raw_schema)` pairs.
/// Returns a list of errors for all incompatible expressions.
pub fn check(
    schemas: &[(String, JsonValue)],
    env: &TypeEnv,
    functions: &FunctionTable,
    interpreter: &dyn SchemaInterpreter,
) -> Vec<TypeError> {
    let mut errors = Vec::new();
    for (name, schema) in schemas {
        if let Err(e) = check_field(name, schema, env, functions, interpreter) {
            errors.push(e);
        }
    }
    errors
}

#[cfg(test)]
mod tests;
