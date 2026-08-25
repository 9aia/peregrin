use handl_dialect::{Schema, SchemaVocabulary, ValidationError};
use handl_vm_values_json::NonFunctionValue;

use super::json_schema::JsonSchema;

/// The canonical schema vocabulary: JSON Schema draft 2020-12.
///
/// Implements [`SchemaVocabulary<NonFunctionValue>`] by compiling raw
/// schema documents into [`JsonSchema`] validators via the
/// [`jsonschema`] crate.
pub struct JsonSchemaVocabulary;

impl SchemaVocabulary<NonFunctionValue> for JsonSchemaVocabulary {
    fn id(&self) -> &'static str {
        "json-schema-draft-2020-12"
    }

    fn compile(
        &self,
        raw: &serde_json::Value,
    ) -> Result<Box<dyn Schema<NonFunctionValue>>, ValidationError> {
        Ok(Box::new(JsonSchema::new(raw.clone())))
    }
}

/// The canonical schema vocabulary as a static singleton.
pub static JSON_SCHEMA_VOCABULARY: JsonSchemaVocabulary = JsonSchemaVocabulary;
