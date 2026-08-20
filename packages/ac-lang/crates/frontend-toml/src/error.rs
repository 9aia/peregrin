/// Errors that can occur during TOML parsing.
#[derive(Debug)]
pub enum TomlError {
    /// The underlying TOML parser returned an error.
    Parse(toml::de::Error),
    /// The underlying TOML serializer returned an error.
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for TomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TomlError::Parse(e) => write!(f, "toml parse error: {e}"),
            TomlError::Serialize(e) => write!(f, "toml serialize error: {e}"),
        }
    }
}

impl std::error::Error for TomlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TomlError::Parse(e) => Some(e),
            TomlError::Serialize(e) => Some(e),
        }
    }
}

impl From<toml::de::Error> for TomlError {
    fn from(e: toml::de::Error) -> Self {
        TomlError::Parse(e)
    }
}

impl From<toml::ser::Error> for TomlError {
    fn from(e: toml::ser::Error) -> Self {
        TomlError::Serialize(e)
    }
}
