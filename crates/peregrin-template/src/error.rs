/// Template resolution failure — the Rust analogue of the TS
/// `Data.TaggedError("TemplateError")`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{reason}")]
pub struct TemplateError {
    /// Human-readable reason, matching the TS error strings.
    pub reason: String,
}

impl TemplateError {
    /// Build an error from any reason string.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}
