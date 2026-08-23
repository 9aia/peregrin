use std::fmt;

use ac_runtime::Error as VmError;

/// Errors produced by HTTP server implementations.
#[derive(Debug, Clone)]
pub enum HttpServerError {
    Network { message: String },
    InvalidArgument { message: String },
    NotRunning,
    AlreadyRunning,
    Handler { message: String },
    NotFound,
}

impl fmt::Display for HttpServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network { message } => write!(f, "network error: {message}"),
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
            Self::NotRunning => write!(f, "server is not running"),
            Self::AlreadyRunning => write!(f, "server is already running"),
            Self::Handler { message } => write!(f, "handler error: {message}"),
            Self::NotFound => write!(f, "not found"),
        }
    }
}

impl std::error::Error for HttpServerError {}

impl From<HttpServerError> for VmError {
    fn from(err: HttpServerError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl HttpServerError {
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}
