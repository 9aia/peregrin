//! # ac-schema-json
//!
//! JSON Schema validation for the AC language.
//!
//! Implements the abstract [`ac_schema::Schema<V>`] trait for
//! `RuntimeValue` using a minimal JSON Schema validator.
//!
//! Supports the `type` and `required` keywords. This is not a
//! complete JSON Schema implementation — it covers the most common
//! cases. Replace with a full validator when needed.
//!
//! ## Architectural rule
//!
//! > **JSON Schema is an implementation, not the abstraction.**

mod schema;
mod validator;

pub use schema::JsonSchema;

#[cfg(test)]
mod validator_test;
