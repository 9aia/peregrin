//! # ac-typecheck-infer
//!
//! Expression type inference for the AC typechecker.
//!
//! Parses expression strings (the same syntax used inside `${...}` and
//! `expr:` fields) and infers their static type given a [`TypeEnv`] and
//! a [`FunctionTable`].
//!
//! ## Architectural rule
//!
//! > **Inference is pure — no I/O, no evaluation, no side effects.**
//!
//! The expression syntax is:
//! - Literals: `"string"`, `123`, `true`, `false`, `null`
//! - Path lookups: `doc.name`, `env.HOME`, `name` (shorthand for `doc.name`)
//! - Function calls: `add(1, 2)`, `mul(x, 2)`, `add(mul(2, 3), 4)`

mod parser;
mod table;

use std::collections::HashMap;

use ac_dialect::Dialect;

pub use parser::{ExprAst, HandlerClauseAst, parse_expr};
pub use table::{FunctionTable, HostFunctionSigs, RawFnSig};

use ac_typecheck_core::{
    CallCheckError, CallErrorKind, FunctionType, TraitRegistry, Type, TypeEnv,
    TypeError, is_compatible, check_bounds, substitute, unify,
};

/// The conventional name of the default namespace used for shorthand
/// path resolution in type inference (bare `name` → `DEFAULT_NAMESPACE.name`).
///
/// This is a runtime host convention, not a language-level requirement.
/// It mirrors `ac_fields::DEFAULT_NAMESPACE` — kept as a separate constant
/// because `ac-typecheck-infer` does not depend on `ac-fields`.
pub const DEFAULT_NAMESPACE: &str = "doc";

/// Infer the type of an expression string.
///
/// Parses the expression, then walks the AST to determine the result type.
/// Returns [`Type::Unknown`] if the type cannot be determined (e.g., an
/// unknown function or an unresolvable path).
pub fn infer(expr: &str, env: &TypeEnv, functions: &FunctionTable, dialect: &dyn Dialect) -> Type {
    let Some(ast) = parse_expr(expr, dialect) else {
        return Type::Unknown;
    };
    infer_ast(&ast, env, functions, dialect)
}

/// Infer the type of an expression string, with AST caching.
///
/// Uses `cache` to avoid re-parsing the same expression string.
/// The cache maps expression strings to their parsed [`ExprAst`].
/// Pass the same `cache` across multiple `infer_cached` calls to
/// eliminate redundant parsing.
pub fn infer_cached(
    expr: &str,
    env: &TypeEnv,
    functions: &FunctionTable,
    cache: &mut HashMap<String, Option<ExprAst>>,
    dialect: &dyn Dialect,
) -> Type {
    let ast = match cache.get(expr) {
        Some(Some(cached)) => Some(cached.clone()),
        Some(None) => None,
        None => {
            let parsed = parse_expr(expr, dialect);
            cache.insert(expr.to_string(), parsed.clone());
            parsed
        }
    };
    let Some(ast) = ast else {
        return Type::Unknown;
    };
    infer_ast(&ast, env, functions, dialect)
}

/// Infer the type of a parsed expression AST.
pub fn infer_ast(
    ast: &ExprAst,
    env: &TypeEnv,
    functions: &FunctionTable,
    dialect: &dyn Dialect,
) -> Type {
    infer_ast_with_expected(ast, env, functions, dialect, None)
}

/// Like [`infer_ast`] but accepts an optional expected type for the expression.
///
/// When the expression is a function call and the expected type is known,
/// this enables return-type-driven inference: the expected type is passed
/// to [`check_call_with_expected`] so that type variables appearing only in
/// the return type can be bound from context.
pub fn infer_ast_with_expected(
    ast: &ExprAst,
    env: &TypeEnv,
    functions: &FunctionTable,
    dialect: &dyn Dialect,
    expected: Option<&Type>,
) -> Type {
    match ast {
        ExprAst::Literal(ty) => ty.clone(),

        ExprAst::Path(path) => resolve_path_type(path, env, dialect),

        ExprAst::Call(name, args) => {
            let arg_types: Vec<Type> = args
                .iter()
                .map(|a| infer_ast(a, env, functions, dialect))
                .collect();

            // Try named function table first.
            if let Some(fn_type) = functions.lookup(name) {
                return check_call_with_expected(fn_type, &arg_types, expected);
            }

            // Try resolving as a path to a function-typed value.
            let ty = resolve_path_type(name, env, dialect);
            if let Type::Function(fn_type) = ty {
                return check_call_with_expected(&fn_type, &arg_types, expected);
            }

            Type::Unknown
        }

        ExprAst::Handle { body, .. } => {
            // The type of a handle expression is the type of its body.
            // Handler clauses may intercept operations but the overall
            // result type is determined by what the body produces.
            infer_ast_with_expected(body, env, functions, dialect, expected)
        }
    }
}

/// Walk a dotted path into a type, returning the type at the end.
fn walk_path(ty: &Type, parts: &[&str]) -> Type {
    if parts.is_empty() {
        return ty.clone();
    }

    if let Type::Object(props) = ty
        && let Some(child_ty) = props.get(parts[0])
    {
        return walk_path(child_ty, &parts[1..]);
    }

    Type::Unknown
}

/// Resolve a dotted path string to a [`Type`] via the [`TypeEnv`].
///
/// Handles both qualified paths (`doc.name`) and shorthand (`name`
/// resolves via the default namespace).
fn resolve_path_type(path: &str, env: &TypeEnv, dialect: &dyn Dialect) -> Type {
    let parts: Vec<&str> = path.split(dialect.syntax().path_separator()).collect();
    if parts.is_empty() {
        return Type::Unknown;
    }

    let first = parts[0];

    if let Some(ty) = env.lookup(first).cloned() {
        return walk_path(&ty, &parts[1..]);
    }

    if let Some(ty) = env.lookup(DEFAULT_NAMESPACE).and_then(|doc_ty| {
        if let Type::Object(props) = doc_ty {
            props.get(first).cloned()
        } else {
            None
        }
    }) {
        return walk_path(&ty, &parts[1..]);
    }

    Type::Unknown
}

/// Check call arguments against a function type, returning the return type
/// or [`Type::Unknown`] if args don't match.
///
/// For monomorphic functions (`type_params` empty), this is a structural
/// compatibility check — same as before.
///
/// For generic functions (`type_params` non-empty), this unifies argument
/// types against parameter types to build a substitution, then applies it
/// to the return type. If unification fails, returns [`Type::Unknown`].
fn check_call(fn_type: &FunctionType, arg_types: &[Type]) -> Type {
    check_call_with_expected(fn_type, arg_types, None)
}

/// Like [`check_call`] but also accepts an optional expected return type.
///
/// When `expected_return` is provided and the function is generic, the
/// expected return type is unified against the declared return type
/// *after* argument unification. This allows type variables that appear
/// only in the return type (not in any parameter) to be bound from the
/// context — this is **return-type-driven inference**.
///
/// For example, given `make_array<T>() -> Array<T>` called in a context
/// expecting `Array<String>`, the expected type `Array<String>` is unified
/// against `Array<T>`, binding `T = String`.
pub fn check_call_with_expected(
    fn_type: &FunctionType,
    arg_types: &[Type],
    expected_return: Option<&Type>,
) -> Type {
    if fn_type.params.len() != arg_types.len() {
        return Type::Unknown;
    }

    if fn_type.type_params.is_empty() {
        for ((_, param_ty), arg_ty) in fn_type.params.iter().zip(arg_types) {
            if !is_compatible(arg_ty, param_ty) {
                return Type::Unknown;
            }
        }
        return (*fn_type.return_type).clone();
    }

    let mut subst = ac_typecheck_core::Substitution::new();
    for ((_, param_ty), arg_ty) in fn_type.params.iter().zip(arg_types) {
        if !unify(arg_ty, param_ty, &mut subst, &fn_type.type_params) {
            return Type::Unknown;
        }
    }

    // Return-type-driven inference: unify expected return against declared
    // return type to bind any type variables that only appear in return position.
    if let Some(expected) = expected_return {
        if !unify(expected, &fn_type.return_type, &mut subst, &fn_type.type_params) {
            // If return-type unification fails, don't fail the whole call —
            // the argument-driven inference may still be correct. Just skip
            // the return-type binding.
            // (This is a graceful degradation — the caller may have an
            // imprecise expected type.)
        }
    }

    // Bind any unbound type params to Unknown (graceful degradation when
    // argument types were Unknown and didn't produce a binding).
    for tp in &fn_type.type_params {
        if !subst.contains_key(&tp.name) {
            subst.insert(tp.name.clone(), Type::Unknown);
        }
    }
    // Check trait bounds using the default registry.
    let registry = TraitRegistry::default_registry();
    if !check_bounds(&subst, &fn_type.type_params, &registry) {
        return Type::Unknown;
    }
    substitute(&fn_type.return_type, &subst)
}

/// Like [`check_call`] but returns detailed error information on mismatch.
///
/// Returns `Ok(Type)` on success, or `Err(CallCheckError)` with a description
/// of what went wrong (wrong arg count, type mismatch, or unification failure).
pub fn check_call_detailed(
    fn_name: &str,
    fn_type: &FunctionType,
    arg_types: &[Type],
) -> Result<Type, CallCheckError> {
    if fn_type.params.len() != arg_types.len() {
        return Err(CallCheckError {
            function: fn_name.to_string(),
            kind: CallErrorKind::ArgCount {
                expected: fn_type.params.len(),
                got: arg_types.len(),
            },
        });
    }

    if fn_type.type_params.is_empty() {
        for ((pname, param_ty), arg_ty) in fn_type.params.iter().zip(arg_types) {
            if !is_compatible(arg_ty, param_ty) {
                return Err(CallCheckError {
                    function: fn_name.to_string(),
                    kind: CallErrorKind::ArgType {
                        param: pname.clone(),
                        index: fn_type.params.iter().position(|(n, _)| n == pname).unwrap_or(0),
                        expected: param_ty.clone(),
                        actual: arg_ty.clone(),
                    },
                });
            }
        }
        return Ok((*fn_type.return_type).clone());
    }

    let mut subst = ac_typecheck_core::Substitution::new();
    for (i, ((pname, param_ty), arg_ty)) in fn_type.params.iter().zip(arg_types).enumerate() {
        if !unify(arg_ty, param_ty, &mut subst, &fn_type.type_params) {
            return Err(CallCheckError {
                function: fn_name.to_string(),
                kind: CallErrorKind::UnificationFailed {
                    param: pname.clone(),
                    index: i,
                    expected: param_ty.clone(),
                    actual: arg_ty.clone(),
                },
            });
        }
    }
    for tp in &fn_type.type_params {
        if !subst.contains_key(&tp.name) {
            subst.insert(tp.name.clone(), Type::Unknown);
        }
    }
    // Check trait bounds using the default registry.
    let registry = TraitRegistry::default_registry();
    if !check_bounds(&subst, &fn_type.type_params, &registry) {
        return Err(CallCheckError {
            function: fn_name.to_string(),
            kind: CallErrorKind::UnificationFailed {
                param: fn_type.type_params.iter().map(|tp| tp.name.clone()).collect::<Vec<_>>().join(", "),
                index: 0,
                expected: Type::Unknown,
                actual: Type::Unknown,
            },
        });
    }
    Ok(substitute(&fn_type.return_type, &subst))
}

/// Like [`infer_ast`] but also collects type errors from function calls.
///
/// Returns `(Type, Vec<TypeError>)` — the inferred type and any call errors
/// found in the expression tree. The inferred type may still be `Unknown`
/// even when errors are present (e.g., a nested call failed but the outer
/// expression continues).
pub fn infer_ast_detailed(
    ast: &ExprAst,
    env: &TypeEnv,
    functions: &FunctionTable,
    dialect: &dyn Dialect,
) -> (Type, Vec<TypeError>) {
    let mut errors = Vec::new();
    let ty = infer_ast_inner_detailed(ast, env, functions, dialect, &mut errors);
    (ty, errors)
}

fn infer_ast_inner_detailed(
    ast: &ExprAst,
    env: &TypeEnv,
    functions: &FunctionTable,
    dialect: &dyn Dialect,
    errors: &mut Vec<TypeError>,
) -> Type {
    match ast {
        ExprAst::Literal(ty) => ty.clone(),

        ExprAst::Path(path) => resolve_path_type(path, env, dialect),

        ExprAst::Call(name, args) => {
            let arg_types: Vec<Type> = args
                .iter()
                .map(|a| infer_ast_inner_detailed(a, env, functions, dialect, errors))
                .collect();

            if let Some(fn_type) = functions.lookup(name) {
                return match check_call_detailed(name, fn_type, &arg_types) {
                    Ok(ty) => ty,
                    Err(e) => {
                        errors.push(e.to_type_error());
                        Type::Unknown
                    }
                };
            }

            let ty = resolve_path_type(name, env, dialect);
            if let Type::Function(fn_type) = ty {
                return match check_call_detailed(name, &fn_type, &arg_types) {
                    Ok(ty) => ty,
                    Err(e) => {
                        errors.push(e.to_type_error());
                        Type::Unknown
                    }
                };
            }

            Type::Unknown
        }

        ExprAst::Handle { body, .. } => {
            infer_ast_inner_detailed(body, env, functions, dialect, errors)
        }
    }
}

/// Like [`infer_cached`] but also collects type errors from function calls.
pub fn infer_cached_detailed(
    expr: &str,
    env: &TypeEnv,
    functions: &FunctionTable,
    cache: &mut HashMap<String, Option<ExprAst>>,
    dialect: &dyn Dialect,
) -> (Type, Vec<TypeError>) {
    let ast = match cache.get(expr) {
        Some(Some(cached)) => Some(cached.clone()),
        Some(None) => None,
        None => {
            let parsed = parse_expr(expr, dialect);
            cache.insert(expr.to_string(), parsed.clone());
            parsed
        }
    };
    let Some(ast) = ast else {
        return (Type::Unknown, Vec::new());
    };
    infer_ast_detailed(&ast, env, functions, dialect)
}

#[cfg(test)]
mod lib_tests;
