use super::schema::Schema;
use super::validation_error::ValidationError;

/// A schema vocabulary — compiles raw schema documents into validators.
///
/// This trait abstracts which schema language a dialect uses. The
/// canonical implementation is JSON Schema draft 2020-12, provided by
/// `handl-dialect-canonical` as `JsonSchemaVocabulary`.
///
/// Generic over `V` so the abstraction works with any value type,
/// not just `NonFunctionValue`.
pub trait SchemaVocabulary<V>: Send + Sync {
    /// The vocabulary identifier (e.g. `"json-schema-draft-2020-12"`).
    fn id(&self) -> &'static str;

    /// Compile a raw schema document into a validator.
    ///
    /// Returns a boxed [`Schema<V>`] that can validate values, or a
    /// [`ValidationError`] if the schema document itself is invalid.
    fn compile(&self, raw: &serde_json::Value) -> Result<Box<dyn Schema<V>>, ValidationError>;
}
