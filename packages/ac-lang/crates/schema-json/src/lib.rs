//! # ac-schema-json
//!
//! JSON Schema validation for the AC language.
//!
//! Implements the abstract [`ac_schema::Schema<V>`] and
//! [`ac_schema::SchemaVocabulary<V>`] traits for `RuntimeValue` using
//! the [`jsonschema`](https://crates.io/crates/jsonschema) crate, which
//! provides full JSON Schema draft 2020-12 support.
//!
//! ## Architectural rule
//!
//! > **JSON Schema is an implementation, not the abstraction.**

mod schema;

pub use schema::{JsonSchema, JsonSchemaVocabulary, JSON_SCHEMA_VOCABULARY};

#[cfg(test)]
mod validator_tests;
