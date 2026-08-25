use std::fmt;

use handl_vm_values_json::Error as VmError;

#[derive(Debug, Clone)]
pub enum TestError {
    ArgumentCount {
        expected: usize,
        got: usize,
    },
    TypeMismatch {
        expected: &'static str,
        got: String,
    },
    AssertionFailed {
        message: String,
    },
    PropertyFailed {
        counterexample: serde_json::Value,
        message: String,
    },
    FuzzFailure {
        run: usize,
        message: String,
    },
    SnapshotMismatch {
        name: String,
        actual: serde_json::Value,
        expected: serde_json::Value,
    },
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArgumentCount { expected, got } => {
                write!(f, "expected {expected} arguments, got {got}")
            }
            Self::TypeMismatch { expected, got } => {
                write!(f, "expected {expected}, got {got}")
            }
            Self::AssertionFailed { message } => f.write_str(message),
            Self::PropertyFailed {
                counterexample,
                message,
            } => {
                write!(f, "property failed at {counterexample}: {message}")
            }
            Self::FuzzFailure { run, message } => {
                write!(f, "fuzz failure on run {run}: {message}")
            }
            Self::SnapshotMismatch {
                name,
                actual,
                expected,
            } => {
                write!(
                    f,
                    "snapshot {name} mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for TestError {}

impl From<TestError> for VmError {
    fn from(err: TestError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}
