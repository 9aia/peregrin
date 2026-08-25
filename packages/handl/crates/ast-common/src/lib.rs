//! # handl-ast-common
//!
//! Common, reusable AST toolkit built on `handl-ast`.
//!
//! Re-exports the generic abstractions from `handl-ast` and adds the most
//! commonly needed metadata types: `Span`, `Attrs`, and `Both`.

pub use handl_ast::*;

pub mod metadata;

pub use metadata::{Attrs, Both, Span};

#[cfg(test)]
mod tests;
