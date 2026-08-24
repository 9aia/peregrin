use ac_ast_common::StmtFactory;

use crate::Expr;

/// A statement in the AC language.
///
/// Statements are top-level program elements that do not produce a value
/// themselves, but contain expressions that do. `Stmt` is generic over the
/// value type `V` so it can share the same value representation as `Expr`.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<V> {
    /// An expression used as a statement.
    Expression(Expr<V>),

    /// A name bound to an expression value.
    Let {
        /// The bound name.
        name: String,

        /// The value assigned to the name.
        value: Expr<V>,
    },

    /// A sequence of statements evaluated in order.
    Block(Vec<Stmt<V>>),
}

impl<V> Stmt<V> {
    /// Creates an expression statement.
    pub fn expression(expr: Expr<V>) -> Self {
        Stmt::Expression(expr)
    }

    /// Creates a `let` statement.
    pub fn let_(name: impl Into<String>, value: Expr<V>) -> Self {
        Stmt::Let {
            name: name.into(),
            value,
        }
    }

    /// Creates a block of statements.
    pub fn block(stmts: Vec<Stmt<V>>) -> Self {
        Stmt::Block(stmts)
    }
}

/// The canonical AC statement factory.
#[derive(Debug, Default, Clone, Copy)]
pub struct CanonicalStmtFactory;

impl<V> StmtFactory<Stmt<V>, Expr<V>> for CanonicalStmtFactory {
    fn expression(&self, expr: Expr<V>) -> Stmt<V> {
        Stmt::expression(expr)
    }

    fn let_<N>(&self, name: N, value: Expr<V>) -> Stmt<V>
    where
        N: Into<String>,
    {
        Stmt::let_(name, value)
    }

    fn block(&self, stmts: Vec<Stmt<V>>) -> Stmt<V> {
        Stmt::block(stmts)
    }
}
