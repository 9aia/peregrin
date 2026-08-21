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

pub use parser::{parse_expr, ExprAst};
pub use table::{FunctionTable, RawFnSig};

use ac_typecheck_core::{is_compatible, FunctionType, Type, TypeEnv};

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
pub fn infer_ast(ast: &ExprAst, env: &TypeEnv, functions: &FunctionTable, dialect: &dyn Dialect) -> Type {
    match ast {
        ExprAst::Literal(ty) => ty.clone(),

        ExprAst::Path(path) => {
            resolve_path_type(path, env, dialect)
        }

        ExprAst::Call(name, args) => {
            let arg_types: Vec<Type> = args
                .iter()
                .map(|a| infer_ast(a, env, functions, dialect))
                .collect();

            // Try named function table first.
            if let Some(fn_type) = functions.lookup(name) {
                return check_call(&fn_type, &arg_types);
            }

            // Try resolving as a path to a function-typed value.
            let ty = resolve_path_type(name, env, dialect);
            if let Type::Function(fn_type) = ty {
                return check_call(&fn_type, &arg_types);
            }

            Type::Unknown
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
fn check_call(fn_type: &FunctionType, arg_types: &[Type]) -> Type {
    if fn_type.params.len() != arg_types.len() {
        return Type::Unknown;
    }

    for ((_, param_ty), arg_ty) in fn_type.params.iter().zip(arg_types) {
        if !is_compatible(arg_ty, param_ty) {
            return Type::Unknown;
        }
    }

    (*fn_type.return_type).clone()
}

#[cfg(test)]
mod lib_tests;
