//! # ac-ast-common
//!
//! Generic, language-agnostic AST machinery.
//!
//! This crate defines the building blocks for composing ASTs: a pluggable
//! `Node` type, a `NodeMetadata` trait, and factory traits for expressions,
//! statements, and types. It has no language semantics and no dependencies.
//!
//! The runtime can use `Node<K, V, ()>` and pay zero bytes for metadata.
//! The parser, typechecker, or any tool can plug in `Span`, `Attrs`, or a
//! composed `Both<...>` metadata type instead.

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
