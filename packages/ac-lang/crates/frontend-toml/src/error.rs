use thiserror::Error;

/// Errors that can occur during TOML parsing.
#[derive(Debug, Error)]
pub enum TomlError {
    /// The underlying TOML parser returned an error.
    #[error("toml parse error: {0}")]
    Parse(#[from] toml::de::Error),

    /// The underlying TOML serializer returned an error.
    #[error("toml serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}