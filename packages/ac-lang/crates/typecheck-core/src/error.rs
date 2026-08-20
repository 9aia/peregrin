use crate::Type;

/// A type error detected during static checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    /// The field name where the error was detected.
    pub field: String,
    /// The expected type, if known.
    pub expected: Option<Type>,
    /// The actual (inferred) type, if known.
    pub actual: Option<Type>,
    /// Human-readable explanation.
    pub message: String,
    /// Dotted path to the location within the document (e.g. `config.server`).
    pub path: String,
}

impl TypeError {
    /// Create a type mismatch error.
    pub fn mismatch(
        field: impl Into<String>,
        expected: Type,
        actual: Type,
        path: impl Into<String>,
    ) -> Self {
        let field = field.into();
        let path = path.into();
        TypeError {
            message: format!(
                "field '{field}' at '{path}': expected {}, got {}",
                expected.name(),
                actual.name()
            ),
            field,
            expected: Some(expected),
            actual: Some(actual),
            path,
        }
    }

    /// Create a generic error with a custom message.
    pub fn custom(field: impl Into<String>, message: impl Into<String>, path: impl Into<String>) -> Self {
        TypeError {
            field: field.into(),
            expected: None,
            actual: None,
            message: message.into(),
            path: path.into(),
        }
    }

    /// Format the error for display.
    pub fn display(&self) -> String {
        if self.path.is_empty() {
            format!("{}: {}", self.field, self.message)
        } else {
            format!("{}.{}: {}", self.path, self.field, self.message)
        }
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl std::error::Error for TypeError {}

/// Result type for typechecking — collects multiple errors.
pub type TypeResult<T> = Result<T, Vec<TypeError>>;
