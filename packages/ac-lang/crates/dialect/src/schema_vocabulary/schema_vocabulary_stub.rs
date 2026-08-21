use ac_runtime::RuntimeValue;

use crate::schema_vocabulary::{Schema, SchemaVocabulary};
use crate::schema_vocabulary::schema_stub::StubSchema;

/// A minimal schema vocabulary for testing.
pub(crate) struct StubVocabulary;

impl SchemaVocabulary<RuntimeValue> for StubVocabulary {
    fn id(&self) -> &'static str {
        "stub"
    }

    fn compile(
        &self,
        _raw: &serde_json::Value,
    ) -> Result<Box<dyn Schema<RuntimeValue>>, crate::schema_vocabulary::ValidationError> {
        Ok(Box::new(StubSchema))
    }
}
