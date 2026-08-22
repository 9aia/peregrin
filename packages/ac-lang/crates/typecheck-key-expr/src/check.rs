use std::collections::HashMap;

use ac_dialect_canonical::AC_DIALECT;
use ac_typecheck_core::{SchemaInterpreter, Type, TypeEnv, TypeError, is_compatible};
use ac_typecheck_infer::{FunctionTable, ParseCache, infer_cached_detailed};
use serde_json::Value as JsonValue;

/// The AC key this crate handles.
pub const KEY: &str = "expr";

/// Get the expression string from a field schema, if present.
pub fn get_expr(schema: &JsonValue) -> Option<String> {
    schema
        .get(KEY)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Check a single field's `expr` against its declared type.
///
/// Returns a [`TypeError`] if the inferred type is incompatible with
/// the declared type. Returns `Ok(())` if the expression is absent or
/// the types are compatible.
#[allow(clippy::result_large_err)]
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
    let mut cache = HashMap::new();
    let (inferred, mut call_errors) =
        infer_cached_detailed(&expr_str, env, functions, &mut cache, &AC_DIALECT);

    if !inferred.is_unknown() && !is_compatible(&inferred, &declared) {
        return Err(TypeError::mismatch(field_name, declared, inferred, ""));
    }

    if !call_errors.is_empty() {
        return Err(call_errors.remove(0));
    }

    Ok(())
}

/// Check a single field's `expr` with an AST cache and a pre-interpreted
/// declared type.
///
/// Like [`check_field`] but reuses a parse cache across multiple calls
/// and accepts the already-interpreted declared type, avoiding redundant
/// schema interpretation.
#[allow(clippy::result_large_err)]
pub fn check_field_cached(
    field_name: &str,
    schema: &JsonValue,
    env: &TypeEnv,
    functions: &FunctionTable,
    interpreter: &dyn SchemaInterpreter,
    cache: &mut ParseCache,
) -> Result<(), TypeError> {
    let Some(expr_str) = get_expr(schema) else {
        return Ok(());
    };

    let declared = interpreter.interpret(schema);
    let (inferred, mut call_errors) =
        infer_cached_detailed(&expr_str, env, functions, cache, &AC_DIALECT);

    if !inferred.is_unknown() && !is_compatible(&inferred, &declared) {
        return Err(TypeError::mismatch(field_name, declared, inferred, ""));
    }

    if !call_errors.is_empty() {
        return Err(call_errors.remove(0));
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
    let mut parse_cache: ParseCache = HashMap::new();
    let mut type_cache: HashMap<String, Type> = HashMap::new();
    for (name, schema) in schemas {
        let Some(expr_str) = get_expr(schema) else {
            continue;
        };
        let declared = type_cache
            .entry(name.clone())
            .or_insert_with(|| interpreter.interpret(schema))
            .clone();
        let (inferred, mut call_errors) =
            infer_cached_detailed(&expr_str, env, functions, &mut parse_cache, &AC_DIALECT);
        if !inferred.is_unknown() && !is_compatible(&inferred, &declared) {
            errors.push(TypeError::mismatch(name, declared, inferred, ""));
            continue;
        }
        if !call_errors.is_empty() {
            errors.push(call_errors.remove(0));
        }
    }
    errors
}
