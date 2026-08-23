use thiserror::Error;

/// Errors that can occur during YAML parsing.
#[derive(Debug, Error)]
pub enum YamlError {
    /// The underlying YAML parser returned an error.
    #[error("yaml parse error: {0}")]
    Parse(#[from] serde_yaml::Error),

    /// Conversion to the core value model failed.
    #[error("yaml conversion error: {0}")]
    Convert(String)}
