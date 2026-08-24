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

mod check;

pub use check::{KEY, check, check_field, check_field_cached, get_expr};

#[cfg(test)]
mod lib_tests;