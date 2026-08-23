//! # ac-ast
//!
//! Base AST machinery for the AC language.
//!
//! This crate defines the generic building blocks: a pluggable `Node` type,
//! a `NodeMetadata` trait, and factory traits for expressions, statements,
//! and types. Concrete canonical AST nodes live in `ac-ast-canonical`.

pub mod expr;
pub mod metadata;
pub mod node;
pub mod stmt;
pub mod type_;

pub use expr::ExprFactory;
pub use metadata::{Attrs, Both, NodeMetadata, Span};
pub use node::Node;
pub use stmt::StmtFactory;
pub use type_::TypeFactory;

#[cfg(test)]
mod tests;
