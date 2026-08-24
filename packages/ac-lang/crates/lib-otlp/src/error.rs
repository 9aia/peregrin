use ac_runtime_values::Error as VmError;
use thiserror::Error;

/// Errors that can occur inside the OTLP module.
#[derive(Debug, Clone, Error)]
pub enum OtlpError {
    /// No exporter endpoint has been configured yet.
    #[error("not configured")]
    NotConfigured,
    /// An argument was malformed or missing.
    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },
    /// The referenced span id does not exist.
    #[error("unknown span: {id}")]
    UnknownSpan { id: String },
    /// The network request failed.
    #[error("network error: {message}")]
    Network { message: String },
    /// Serialization of the OTLP payload failed.
    #[error("serialization error: {message}")]
    Serialization { message: String },
}

impl OtlpError {
    /// Convenience constructor for [`OtlpError::InvalidArgument`].
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
        }
    }
    /// Convenience constructor for [`OtlpError::Network`].
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }
    /// Convenience constructor for [`OtlpError::Serialization`].
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }
}

impl From<OtlpError> for VmError {
    fn from(err: OtlpError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

/// Result type used inside the OTLP module.
pub type OtlpResult<T> = Result<T, OtlpError>;
