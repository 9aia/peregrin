//! # handl-ast
//!
//! Generic AST abstractions for the handl language.
//!
//! Defines the pluggable `Node` type, the `NodeMetadata` trait, and factory
//! traits for expressions, statements, and types. Concrete handl nodes live
//! in `handl-ast-canonical`; common reusable metadata lives in `handl-ast-common`.

mod expr;
mod metadata;
mod node;
mod stmt;
mod type_;

pub use expr::ExprFactory;
pub use metadata::NodeMetadata;
pub use node::Node;
pub use stmt::StmtFactory;
pub use type_::TypeFactory;
