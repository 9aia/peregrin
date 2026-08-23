use std::fmt;

use ac_runtime::Error as VmError;

#[derive(Debug, Clone)]
pub enum CryptoError {
    InvalidInput { message: String },
    InvalidByteValue { value: f64 },
    DecodeError { message: String },
    InvalidArgument { message: String },
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { message } => write!(f, "invalid input: {message}"),
            Self::InvalidByteValue { value } => {
                write!(f, "invalid byte value: {value} (must be 0–255)")
            }
            Self::DecodeError { message } => write!(f, "decode error: {message}"),
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<CryptoError> for VmError {
    fn from(err: CryptoError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl CryptoError {
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}
