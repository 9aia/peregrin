//! # ac-ast
//!
//! A thin facade over `ac-ast-common` and `ac-ast-canonical`.
//! This preserves the existing public API while the implementation
//! lives in the more focused domain crates.

pub use ac_ast_canonical::{
    CanonicalExprFactory, CanonicalStmtFactory, CanonicalTypeFactory, Expr, Field, HandlerClause,
    Stmt, Type,
};
pub use ac_ast_common::{
    Attrs, Both, ExprFactory, Node, NodeMetadata, Span, StmtFactory, TypeFactory,
};
