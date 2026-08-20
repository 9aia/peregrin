use std::collections::HashMap;

use ac_typecheck_core::{is_compatible, SchemaInterpreter, TypeEnv, TypeError};
use ac_typecheck_infer::{infer_cached, FunctionTable};
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
    let mut cache = HashMap::new();
    let inferred = infer_cached(&expr_str, env, functions, &mut cache);

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

/// Check a single field's `expr` with an AST cache.
///
/// Like [`check_field`] but reuses a parse cache across multiple calls
/// to avoid re-parsing the same expression strings.
pub fn check_field_cached(
    field_name: &str,
    schema: &JsonValue,
    env: &TypeEnv,
    functions: &FunctionTable,
    interpreter: &dyn SchemaInterpreter,
    cache: &mut HashMap<String, Option<ac_typecheck_infer::ExprAst>>,
) -> Result<(), TypeError> {
    let Some(expr_str) = get_expr(schema) else {
        return Ok(());
    };

    let declared = interpreter.interpret(schema);
    let inferred = infer_cached(&expr_str, env, functions, cache);

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
    let mut cache: HashMap<String, Option<ac_typecheck_infer::ExprAst>> = HashMap::new();
    for (name, schema) in schemas {
        if let Err(e) = check_field_cached(name, schema, env, functions, interpreter, &mut cache) {
            errors.push(e);
        }
    }
    errors
}
