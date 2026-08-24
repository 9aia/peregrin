//! # ac-ast
//!
//! Generic AST abstractions for the AC language.
//!
//! Defines the pluggable `Node` type, the `NodeMetadata` trait, and factory
//! traits for expressions, statements, and types. Concrete AC nodes live
//! in `ac-ast-canonical`; common reusable metadata lives in `ac-ast-common`.

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
