//! # ac-typegen-json
//!
//! JSON Schema backend for the AC typegen system.
//!
//! Implements the [`TypegenBackend`] trait from `ac-typegen` and produces
//! JSON Schema documents (draft-07) from clean field schema fragments.
//!
//! ## Architectural rule
//!
//! > **The backend interprets the vocabulary; the core provides the data.**

mod compose;
mod emit;

use serde_json::Value as JsonValue;

use ac_typegen::{TypegenBackend, TypegenContext};

pub use compose::compose_schema;
pub use emit::{write_schema_file, inject_schema_comment};

/// JSON Schema backend. Produces `JsonValue` output.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonSchemaBackend;

impl TypegenBackend for JsonSchemaBackend {
    type Output = JsonValue;

    fn generate(&self, ctx: &TypegenContext) -> JsonValue {
        compose_schema(&ctx.fields)
    }

    fn extension(&self) -> &str {
        "schema.json"
    }

    fn ide_hint(&self, schema_rel_path: &str) -> String {
        format!("# yaml-language-server: $schema={schema_rel_path}")
    }
}

// Tests are inline in compose.rs and emit.rs.
// End-to-end tests are in end_to_end_test.rs.
#[cfg(test)]
mod end_to_end_test;
