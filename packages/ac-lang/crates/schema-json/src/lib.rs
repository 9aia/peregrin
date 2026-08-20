//! # ac-schema-json
//!
//! JSON Schema validation for the AC language.
//!
//! Implements the abstract [`ac_schema::Schema<V>`] trait for
//! `RuntimeValue` using the [`jsonschema`](https://crates.io/crates/jsonschema)
//! crate, which provides full JSON Schema draft 2020-12 support.
//!
//! ## Architectural rule
//!
//! > **JSON Schema is an implementation, not the abstraction.**

mod schema;

pub use schema::JsonSchema;

#[cfg(test)]
mod validator_tests;
