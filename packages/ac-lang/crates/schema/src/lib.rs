//! # ac-schema
//!
//! Schema abstraction for the AC language.
//!
//! This crate defines a generic [`Schema<V>`] trait and a
//! [`ValidationError`] type. It has zero dependencies — it does
//! not know about JSON Schema, `serde_json`, or any specific
//! validation backend.
//!
//! Concrete implementations live in separate crates:
//!
//! - `ac-schema-json` — JSON Schema validation for `RuntimeValue`.
//!
//! ## Architectural rule
//!
//! > **`schema` gives values meaning.**
//!
//! ```text
//! ┌───────────────┐
//! │      VM       │  computation
//! └───────┬───────┘
//!         │
//!         ▼
//! ┌───────────────┐
//! │ RuntimeValue  │  data
//! └───────┬───────┘
//!         │
//!         ▼
//! ┌───────────────┐
//! │    Schema     │  type/shape semantics
//! └───────┬───────┘
//!         │
//!    ┌────┴─────┐
//!    ▼          ▼
//!  JSON       Future
//!  Schema     Schema
//! ```

mod schema;

pub use schema::{Schema, ValidationError};

#[cfg(test)]
mod schema_tests;
