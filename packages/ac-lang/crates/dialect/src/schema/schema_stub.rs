use ac_runtime::RuntimeValue;

use crate::schema::{Schema, SchemaVocabulary, ValidationError};

/// A minimal schema vocabulary for testing.
pub(crate) struct StubVocabulary;

impl SchemaVocabulary<RuntimeValue> for StubVocabulary {
    fn id(&self) -> &'static str {
        "stub"
    }

    fn compile(
        &self,
        _raw: &serde_json::Value,
    ) -> Result<Box<dyn Schema<RuntimeValue>>, ValidationError> {
        Err(ValidationError {
            message: "not implemented".to_string(),
            path: String::new(),
            location: None,
        })
    }
}
