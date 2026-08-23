use std::fmt;

use ac_runtime::Error as VmError;

/// Errors produced by key-value store implementations.
#[derive(Debug, Clone)]
pub enum KvError {
    KeyNotFound { key: String },
    Backend { message: String },
}

impl fmt::Display for KvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyNotFound { key } => write!(f, "key not found: {key}"),
            Self::Backend { message } => write!(f, "kv backend error: {message}"),
        }
    }
}

impl std::error::Error for KvError {}

impl From<KvError> for VmError {
    fn from(err: KvError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}
