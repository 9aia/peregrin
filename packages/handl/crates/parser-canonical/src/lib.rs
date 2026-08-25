//! # handl-parser-canonical
//!
//! Canonical recursive-descent parser for handl expressions.
//!
//! Implements [`Parser`] using the canonical grammar:
//!
//! ```text
//! expr      := value | call | handle
//! value     := string | number | byte | bool | null
//! byte      := "0x" hex_digit{1,2}
//! call      := identifier "(" args? ")"
//! args      := expr ("," expr)*
//! handle    := "handle" "(" handler-clause ("," handler-clause)* ")"
//! handler   := string "(" params? ")" "as" identifier ":" expr
//! ```
//!
//! The parser consumes tokens from [`CanonicalLexer`] (or any
//! [`Lexer`](handl_lexer::Lexer) producing compatible tokens) and
//! produces an [`Expr`] tree ready for the VM.

mod parser;

pub use parser::CanonicalParser;

#[cfg(test)]
mod tests;
