//! # ac-dialect
//!
//! Dialect abstraction for the AC language implementation.
//!
//! The AC language specification defines one canonical syntax. This crate
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
//! changing the spec. The canonical implementation lives in `ac-dialect-canonical`.

mod dialect;
mod schema;

#[cfg(test)]
mod dialect_tests;

pub use dialect::Dialect;
pub use schema::{Schema, SchemaVocabulary, ValidationError};
