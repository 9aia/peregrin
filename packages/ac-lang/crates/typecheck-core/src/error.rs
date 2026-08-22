use crate::Type;
use ac_source::SourceLocation;

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
    /// Source location (line:column) of the offending value, if known.
    /// Populated by the engine after typechecking, using the original
    /// source text.
    pub location: Option<SourceLocation>,
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
            location: None,
        }
    }

    /// Create a generic error with a custom message.
    pub fn custom(
        field: impl Into<String>,
        message: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        TypeError {
            field: field.into(),
            expected: None,
            actual: None,
            message: message.into(),
            path: path.into(),
            location: None,
        }
    }

    /// Format the error for display.
    pub fn display(&self) -> String {
        let prefix = if let Some(loc) = self.location {
            format!("[{loc}] ")
        } else {
            String::new()
        };
        if self.path.is_empty() {
            format!("{prefix}{}: {}", self.field, self.message)
        } else {
            format!("{prefix}{}.{}: {}", self.path, self.field, self.message)
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

/// A type error detected during function call checking.
///
/// Produced when argument types don't match a function's parameter types,
/// including generic unification failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallCheckError {
    /// The function name being called.
    pub function: String,
    /// What went wrong.
    pub kind: CallErrorKind,
}

/// The kind of call checking error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallErrorKind {
    /// Wrong number of arguments.
    ArgCount { expected: usize, got: usize },
    /// Argument type incompatible with parameter type (monomorphic).
    ArgType {
        param: String,
        index: usize,
        expected: Type,
        actual: Type,
    },
    /// Generic unification failed — a type variable couldn't be bound
    /// consistently.
    UnificationFailed {
        param: String,
        index: usize,
        expected: Type,
        actual: Type,
    },
}

impl CallCheckError {
    /// Build a `TypeError` from this call error.
    pub fn to_type_error(&self) -> TypeError {
        let msg = match &self.kind {
            CallErrorKind::ArgCount { expected, got } => {
                format!(
                    "call to '{}': expected {} argument(s), got {}",
                    self.function, expected, got
                )
            }
            CallErrorKind::ArgType {
                param,
                index,
                expected,
                actual,
            } => {
                format!(
                    "call to '{}': argument {} ('{}'): expected {}, got {}",
                    self.function,
                    index + 1,
                    param,
                    expected.name(),
                    actual.name()
                )
            }
            CallErrorKind::UnificationFailed {
                param,
                index,
                expected,
                actual,
            } => {
                format!(
                    "call to '{}': argument {} ('{}'): type mismatch in generic parameter — expected {}, got {}",
                    self.function,
                    index + 1,
                    param,
                    expected.name(),
                    actual.name()
                )
            }
        };
        TypeError::custom(self.function.clone(), msg, "")
    }
}
