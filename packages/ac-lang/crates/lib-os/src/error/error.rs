use std::fmt;
use ac_runtime_values::Error as VmError;

/// Errors produced by OS service implementations.
///
/// Converts to [`ac_runtime::Error`] via `From`, so service methods can return
/// `Result<T, OsError>` and the adapter handles the conversion.
#[derive(Debug, Clone)]
pub enum OsError {
    NotFound { path: String },
    PermissionDenied { path: String },
    AlreadyExists { path: String },
    InvalidArgument { message: String },
    Io { message: String },
}

impl fmt::Display for OsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "not found: {path}"),
            Self::PermissionDenied { path } => write!(f, "permission denied: {path}"),
            Self::AlreadyExists { path } => write!(f, "already exists: {path}"),
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
            Self::Io { message } => write!(f, "io error: {message}"),
        }
    }
}

impl std::error::Error for OsError {}

impl From<OsError> for VmError {
    fn from(err: OsError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl OsError {
    /// Converts into a [`VmError`].
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}