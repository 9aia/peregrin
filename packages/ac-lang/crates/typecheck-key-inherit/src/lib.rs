//! # ac-typecheck-key-inherit
//!
//! Typechecks the `inheritable` AC key.
//!
//! Fields with `inheritable: true` propagate their values into nested
//! objects. This crate checks that override values in nested scopes have
//! types compatible with the parent declaration.
//!
//! ## Architectural rule
//!
//! > **Inherited field overrides must be type-compatible with the
//! > parent declaration.**

mod walker;
mod check;

pub use check::{check, is_inheritable, KEY};

#[cfg(test)]
mod lib_tests;
