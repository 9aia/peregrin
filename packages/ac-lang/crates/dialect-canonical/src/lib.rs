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
//! This is the only shipped dialect. See [SPEC.md](../dialect/SPEC.md)
//! for the full dialect specification.

#![allow(clippy::module_inception)]

mod dialect;
mod document_vocabulary;
mod expr_parser;
mod schema_vocabulary;
mod syntax;

pub use dialect::{AC_DIALECT, AcDialect};
pub use schema_vocabulary::{JSON_SCHEMA_VOCABULARY, JsonSchema, JsonSchemaVocabulary};
