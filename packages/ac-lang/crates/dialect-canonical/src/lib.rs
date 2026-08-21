//! # ac-dialect-canonical
//!
//! The canonical AC language dialect implementation.
//!
//! Implements the [`Dialect`] trait with the syntax defined in the AC
//! Language Specification:
//! - Interpolation: `${...}`
//! - Path separator: `.`
//! - Function calls: `fn(args)`
//! - String literals: `"..."` or `'...'`
//! - Number literals: `42`, `1.5`, `-3`
//! - Boolean literals: `true`, `false`
//! - Null literal: `null`
//! - Schema vocabulary: JSON Schema draft 2020-12
//!
//! This is the only shipped dialect. See [DIALECT.md](../../docs/DIALECT.md)
//! for the full dialect specification.

mod dialect;
mod expr_parser;
mod schema_vocabulary;
mod syntax;
mod document_vocabulary;

pub use dialect::{AcDialect, AC_DIALECT};
pub use schema_vocabulary::{JsonSchema, JsonSchemaVocabulary, JSON_SCHEMA_VOCABULARY};
