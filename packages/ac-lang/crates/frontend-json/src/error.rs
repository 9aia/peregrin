use thiserror::Error;

/// Errors that can occur during JSON parsing.
#[derive(Debug, Error)]
pub enum JsonError {
    /// The underlying JSON parser returned an error.
    #[error("json parse error: {0}")]
    Parse(#[from] serde_json::Error),
}
