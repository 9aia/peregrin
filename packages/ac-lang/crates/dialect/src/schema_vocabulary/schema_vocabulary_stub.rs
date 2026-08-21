use ac_runtime::NonFunctionValue;

use crate::schema_vocabulary::schema_stub::StubSchema;
use crate::schema_vocabulary::{Schema, SchemaVocabulary};

/// A minimal schema vocabulary for testing.
pub struct StubVocabulary;

impl SchemaVocabulary<NonFunctionValue> for StubVocabulary {
    fn id(&self) -> &'static str {
        "stub"
    }

    fn compile(
        &self,
        _raw: &serde_json::Value,
    ) -> Result<Box<dyn Schema<NonFunctionValue>>, crate::schema_vocabulary::ValidationError> {
        Ok(Box::new(StubSchema))
    }
}
