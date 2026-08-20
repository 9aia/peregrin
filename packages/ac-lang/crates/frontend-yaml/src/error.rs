/// Errors that can occur during YAML parsing.
#[derive(Debug)]
pub enum YamlError {
    /// The underlying YAML parser returned an error.
    Parse(serde_yaml::Error),
    /// Conversion to the core value model failed.
    Convert(String),
}

impl std::fmt::Display for YamlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlError::Parse(e) => write!(f, "yaml parse error: {e}"),
            YamlError::Convert(msg) => write!(f, "yaml conversion error: {msg}"),
        }
    }
}

impl std::error::Error for YamlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            YamlError::Parse(e) => Some(e),
            YamlError::Convert(_) => None,
        }
    }
}

impl From<serde_yaml::Error> for YamlError {
    fn from(e: serde_yaml::Error) -> Self {
        YamlError::Parse(e)
    }
}
