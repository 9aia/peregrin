/// A validation error.
///
/// Carries a human-readable message and a dotted path to the
/// offending value within the document (empty string for root).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Human-readable description of the violation.
    pub message: String,

    /// Dotted path to the offending value within the document.
    /// Empty string for the root.
    pub path: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates values of type `V`.
///
/// This trait is the abstract schema interface. It knows nothing
/// about JSON Schema, `$ref`, `allOf`, `format`, or any other
/// concrete schema language. Those concepts belong to
/// implementations.
///
/// Generic over `V` so the abstraction works with any value type,
/// not just `RuntimeValue`.
pub trait Schema<V> {
    /// Validates a value against this schema.
    ///
    /// Returns `Ok(())` if the value conforms, or a
    /// [`ValidationError`] describing the first violation.
    fn validate(&self, value: &V) -> Result<(), ValidationError>;
}
