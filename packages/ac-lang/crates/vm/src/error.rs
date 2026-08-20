/// Errors that can occur during evaluation.
///
/// All errors are cloneable and comparable, making them suitable
/// for testing and replay scenarios.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The function name was not recognized by the host.
    UnknownFunction(String),

    /// The variable name was not found in any scope.
    UnknownVariable(String),

    /// A primitive returned an error during execution.
    PrimitiveError(String),

    /// A value did not match the expected type.
    ///
    /// Carries the expected type name and the actual type name for
    /// diagnostics.
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    /// The argument count did not match what the primitive expected.
    ArgumentCount {
        expected: usize,
        got: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownFunction(name) => {
                write!(f, "unknown function: {name}")
            }
            Error::UnknownVariable(name) => {
                write!(f, "unknown variable: {name}")
            }
            Error::PrimitiveError(msg) => {
                write!(f, "primitive error: {msg}")
            }
            Error::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {expected}, got {got}")
            }
            Error::ArgumentCount { expected, got } => {
                write!(f, "argument count mismatch: expected {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for Error {}
