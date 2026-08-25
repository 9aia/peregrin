use thiserror::Error;

/// The shared error type for all serialization failures.
#[derive(Debug, Error)]
#[error("serialization error: {reason}")]
pub struct SerializeError {
    /// Human-readable description of the failure.
    pub reason: String,
}

impl SerializeError {
    /// Create a new serialization error.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}
