//! # ac-parser-canonical
//!
//! Canonical recursive-descent parser for AC expressions.
//!
//! Implements [`Parser`] using the canonical grammar:
//!
//! ```text
//! expr      := value | call | handle
//! value     := string | number | bool | null
//! call      := identifier "(" args? ")"
//! args      := expr ("," expr)*
//! handle    := "handle" "(" handler-clause ("," handler-clause)* ")"
//! handler   := string "(" params? ")" "as" identifier ":" expr
//! ```
//!
//! The parser consumes tokens from [`CanonicalLexer`] (or any
//! [`Lexer`](ac_lexer::Lexer) producing compatible tokens) and
//! produces an [`Expr`] tree ready for the VM.

mod parser;

pub use parser::CanonicalParser;

#[cfg(test)]
mod tests;
