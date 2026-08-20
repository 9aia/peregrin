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

pub use parser::{parse_expr, ExprAst};
pub use table::{FunctionTable, RawFnSig};

use ac_typecheck_core::{is_compatible, Type, TypeEnv};

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
pub fn infer(expr: &str, env: &TypeEnv, functions: &FunctionTable) -> Type {
    let Some(ast) = parse_expr(expr) else {
        return Type::Unknown;
    };
    infer_ast(&ast, env, functions)
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
) -> Type {
    let ast = match cache.get(expr) {
        Some(Some(cached)) => Some(cached.clone()),
        Some(None) => None,
        None => {
            let parsed = parse_expr(expr);
            cache.insert(expr.to_string(), parsed.clone());
            parsed
        }
    };
    let Some(ast) = ast else {
        return Type::Unknown;
    };
    infer_ast(&ast, env, functions)
}

/// Infer the type of a parsed expression AST.
pub fn infer_ast(ast: &ExprAst, env: &TypeEnv, functions: &FunctionTable) -> Type {
    match ast {
        ExprAst::Literal(ty) => ty.clone(),

        ExprAst::Path(path) => {
            let parts: Vec<&str> = path.split('.').collect();
            if parts.is_empty() {
                return Type::Unknown;
            }

            let first = parts[0];
            let lookup = |name: &str| env.lookup(name).cloned();

            if let Some(ty) = lookup(first) {
                walk_path(&ty, &parts[1..])
            } else if let Some(ty) = lookup(DEFAULT_NAMESPACE).and_then(|doc_ty| {
                if let Type::Object(props) = doc_ty {
                    props.get(first).cloned()
                } else {
                    None
                }
            }) {
                walk_path(&ty, &parts[1..])
            } else {
                Type::Unknown
            }
        }

        ExprAst::Call(name, args) => {
            let arg_types: Vec<Type> = args
                .iter()
                .map(|a| infer_ast(a, env, functions))
                .collect();

            let Some(fn_type) = functions.lookup(name) else {
                return Type::Unknown;
            };

            if fn_type.params.len() != arg_types.len() {
                return Type::Unknown;
            }

            for ((_, param_ty), arg_ty) in fn_type.params.iter().zip(&arg_types) {
                if !is_compatible(arg_ty, param_ty) {
                    return Type::Unknown;
                }
            }

            (*fn_type.return_type).clone()
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

#[cfg(test)]
mod lib_tests;
