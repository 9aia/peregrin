//! # ac-parser
//!
//! Parsing abstraction for AC expressions.
//!
//! Defines the [`Parser`] trait and [`ParseError`] — the interface for
//! turning a token stream (from [`Lexer`](ac_lexer::Lexer)) into an
//! [`Expr`](ac_ast::Expr) AST. The canonical implementation lives in
//! `ac-parser-canonical`.
//!
//! ## Architectural rule
//!
//! > **The parser consumes tokens and produces `Expr`.**
//!
//! The parser is the bridge between the lexer and the VM. It depends on
//! `ac-lexer` (for `Token`/`TokenKind`) and `ac-ast` (for `Expr`), but
//! not on the VM. This keeps the parsing pipeline independent of
//! evaluation strategy.

mod error;

pub use error::ParseError;

use ac_ast::Expr;
use ac_lexer::{LexError, Lexer, Token, TokenKind};
use ac_runtime::NonFunctionValue;

/// A parser that converts a token stream into an expression AST.
///
/// The parser consumes the token stream produced by a
/// [`Lexer`](ac_lexer::Lexer) and builds an [`Expr`] tree. Dialect-specific
/// grammar rules (operator precedence, keyword handling, call syntax) are
/// implemented by the concrete type.
///
/// The parser does not perform type checking or resolution — it produces
/// a syntactically valid AST only.
pub trait Parser<K: TokenKind, E: LexError>: Send + Sync {
    /// Parse a token stream into an expression.
    ///
    /// Returns `Err(ParseError)` on syntactic errors (unexpected token,
    /// missing delimiter, etc.).
    fn parse(&self, tokens: Vec<Token<K>>) -> Result<Expr<NonFunctionValue>, ParseError>;

    /// Convenience: lex and parse in one call.
    ///
    /// Default implementation uses the provided [`Lexer`](ac_lexer::Lexer)
    /// to tokenize, then calls [`parse`](Self::parse). Override if the
    /// parser benefits from streaming tokens.
    fn parse_text(
        &self,
        input: &str,
        lexer: &dyn Lexer<K, E>,
    ) -> Result<Expr<NonFunctionValue>, ParseError> {
        let tokens = lexer.lex(input).map_err(|e| ParseError::LexError {
            message: e.to_string(),
        })?;
        self.parse(tokens)
    }
}
