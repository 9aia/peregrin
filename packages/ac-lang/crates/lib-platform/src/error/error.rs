use std::fmt;

use ac_vm::Error as VmError;

/// Errors produced by platform service implementations.
///
/// Converts to [`ac_vm::Error`] via `From`, so service methods can return
/// `Result<T, PlatformError>` and the adapter handles the conversion.
#[derive(Debug, Clone)]
pub enum PlatformError {
    NotFound { path: String },
    PermissionDenied { path: String },
    AlreadyExists { path: String },
    Network { message: String },
    InvalidArgument { message: String },
    Io { message: String }}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "not found: {path}"),
            Self::PermissionDenied { path } => write!(f, "permission denied: {path}"),
            Self::AlreadyExists { path } => write!(f, "already exists: {path}"),
            Self::Network { message } => write!(f, "network error: {message}"),
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
            Self::Io { message } => write!(f, "io error: {message}")}
    }
}

impl std::error::Error for PlatformError {}

impl From<PlatformError> for VmError {
    fn from(err: PlatformError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl PlatformError {
    /// Converts into a [`VmError`].
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}
