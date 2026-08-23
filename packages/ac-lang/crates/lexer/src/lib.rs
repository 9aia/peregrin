//! # ac-lexer
//!
//! Lexing abstraction for AC expressions.
//!
//! Defines the [`Lexer`] trait, [`Token`], [`TokenKind`], [`Span`], and
//! [`LexError`] — the types needed to turn raw expression text into a
//! token stream. The canonical implementation lives in `ac-lexer-canonical`.
//!
//! ## Architectural rule
//!
//! > **The lexer produces tokens; the parser consumes them.**
//!
//! The lexer is deliberately separate from the parser so that tooling
//! (LSP, formatters, linters) can depend on tokens without pulling in
//! the full parser and AST.

mod config;
#[cfg(test)]
mod config_test;
mod error;
mod lexer;
#[cfg(test)]
mod lexer_test;
pub mod primitives;
mod rule;
mod span;
mod token;
#[cfg(test)]
mod token_test;

pub use config::LexerConfig;
pub use error::{DefaultLexError, LexError};
pub use lexer::Lexer;
pub use rule::{FnRule, LexRule};
pub use span::Span;
pub use token::{
    CommentFactory, CommentTokenKind, EofFactory, IdentFactory, KeywordFactory, KindFactory,
    NumberFactory, OperatorFactory, StringFactory, Token, TokenKind,
};
