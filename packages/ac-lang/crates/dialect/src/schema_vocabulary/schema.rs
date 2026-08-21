use super::validation_error::ValidationError;

/// Validates values of type `V`.
///
/// This trait is the abstract schema interface. It knows nothing
/// about JSON Schema, `$ref`, `allOf`, `format`, or any other
/// concrete schema language. Those concepts belong to
/// implementations.
///
/// Generic over `V` so the abstraction works with any value type,
/// not just `NonFunctionValue`.
pub trait Schema<V> {
    /// Validates a value against this schema.
    ///
    /// Returns `Ok(())` if the value conforms, or a
    /// [`ValidationError`] describing the first violation.
    fn validate(&self, value: &V) -> Result<(), ValidationError>;
}

impl<S: Schema<V> + ?Sized, V> Schema<V> for Box<S> {
    fn validate(&self, value: &V) -> Result<(), ValidationError> {
        (**self).validate(value)
    }
}
