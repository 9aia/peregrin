//! # ac-ast-common
//!
//! Common, reusable AST toolkit built on `ac-ast`.
//!
//! Re-exports the generic abstractions from `ac-ast` and adds the most
//! commonly needed metadata types: `Span`, `Attrs`, and `Both`.

pub use ac_ast::*;

pub mod metadata;

pub use metadata::{Attrs, Both, Span};

#[cfg(test)]
mod tests;