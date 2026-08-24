//! # ac-ast-canonical
//!
//! Concrete canonical AC AST nodes and factories.
//!
//! Re-exports the generic `ac-ast` and `ac-ast-common` machinery, plus
//! concrete `Expr`, `Field`, `Stmt`, `Type` and their canonical factories.

pub use ac_ast_common::*;

mod expr;
mod field;
mod stmt;
mod type_;

pub use expr::{CanonicalExprFactory, Expr, HandlerClause, Pattern};
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
