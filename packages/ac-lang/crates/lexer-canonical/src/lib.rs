//! # ac-lexer-canonical
//!
//! Canonical lexer for AC expressions.
//!
//! Implements [`Lexer`] using the canonical AC dialect syntax:
//! - `"..."` and `'...'` for strings (with backslash escapes)
//! - `42`, `1.5`, `-3` for numbers
//! - `true` / `false` for booleans, `null` for null
//! - `handle` and `as` as keywords
//! - `(` `)` `{` `}` `[` `]` `,` `:` `.` as delimiters
//! - `// line comments` and `/* block comments */`
//!
//! Dots are separate `Dot` tokens; `math.sqrt` lexes as `math`, `.`, `sqrt`.
//!
//! The canonical config is available as [`canonical_config()`]; custom
//! dialects can start from it and modify keywords, delimiters, and rules
//! using [`LexerConfig`]'s builder methods.

mod config;
mod kind;
mod lexer;

#[cfg(test)]
mod lexer_test;

pub use config::canonical_config;
pub use kind::{CanonicalFactory, CanonicalKind};
pub use lexer::{CanonicalLexer, TokenIter};
