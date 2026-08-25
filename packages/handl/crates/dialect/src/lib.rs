//! # handl-dialect
//!
//! Dialect abstraction for the handl language implementation.
//!
//! The handl language specification defines one canonical syntax. This crate
//! abstracts the syntax decisions, special key names, and schema vocabulary
//! behind a [`Dialect`] trait so the implementation can be structured around
//! a single source of truth.
//!
//! ## Architectural rule
//!
//! > **The language spec fixes syntax. The implementation abstracts it.**
//!
//! The spec says `${...}` for interpolation, `.` for paths, `fn()` for calls.
//! The `Dialect` trait lets the Rust implementation be parameterized without
//! changing the spec. The canonical implementation lives in `handl-dialect-canonical`.

#![allow(clippy::module_inception)]

pub mod dialect;
mod document_vocabulary;
mod expr_parser;
mod schema_vocabulary;
mod syntax;

pub use dialect::Dialect;
pub use document_vocabulary::DocumentVocabulary;
pub use expr_parser::{ExprParser, ParsedHandlerClause};
pub use schema_vocabulary::{Schema, SchemaVocabulary, ValidationError};
pub use syntax::Syntax;
