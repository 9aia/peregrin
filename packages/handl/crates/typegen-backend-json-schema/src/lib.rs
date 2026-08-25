//! # handl-typegen-json
//!
//! JSON Schema backend for the handl typegen system.
//!
//! Implements the [`TypegenBackend`] trait from `handl-typegen-core` and produces
//! JSON Schema documents (draft-07) from clean field schema fragments.
//!
//! ## Architectural rule
//!
//! > **The backend interprets the vocabulary; the core provides the data.**

mod compose;
mod emit;

use serde_json::Value as JsonValue;

use handl_typegen_core::{TypegenBackend, TypegenContext};

pub use compose::compose_schema;
pub use emit::{inject_schema_comment, write_schema_file};

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

#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod emit_tests;
#[cfg(test)]
mod end_to_end_tests;
