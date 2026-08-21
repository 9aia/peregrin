//! # ac-typecheck-schema-json
//!
//! JSON Schema → [`Type`] mapping for the AC typechecker.
//!
//! Implements the [`SchemaInterpreter`] trait from `ac-typecheck-core`,
//! converting JSON Schema fragments into neutral [`Type`] values.
//!
//! ## Architectural rule
//!
//! > **The interpreter reads the schema vocabulary; the core provides
//! > the type system.**

mod interpret;

use ac_typecheck_core::{SchemaInterpreter, Type};
use serde_json::Value as JsonValue;

pub use interpret::{infer_json_type, interpret_function, interpret_schema};

/// JSON Schema interpreter — converts JSON Schema fragments to [`Type`]s.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonSchemaInterpreter;

impl SchemaInterpreter for JsonSchemaInterpreter {
    fn interpret(&self, schema: &JsonValue) -> Type {
        interpret_schema(schema)
    }
}

#[cfg(test)]
mod lib_tests;
