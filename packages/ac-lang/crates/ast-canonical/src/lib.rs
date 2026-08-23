//! # ac-ast-canonical
//!
//! The canonical AST for the AC language — expressions, handler clauses,
//! and fields. Re-exports the generic `ac-ast-common` machinery so
//! consumers only need one dependency.

pub use ac_ast_common::{
    Attrs, Both, ExprFactory, Node, NodeMetadata, Span, StmtFactory, TypeFactory,
};

mod expr;
mod field;
mod stmt;
mod type_;

pub use expr::{CanonicalExprFactory, Expr, HandlerClause};
pub use field::Field;
pub use stmt::{CanonicalStmtFactory, Stmt};
pub use type_::{CanonicalTypeFactory, Type};

#[cfg(test)]
mod expr_tests;

#[cfg(test)]
mod factory_tests;

#[cfg(test)]
mod field_tests;

#[cfg(test)]
mod stmt_tests;

#[cfg(test)]
mod type_tests;
