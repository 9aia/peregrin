use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CompressError {
    #[error("compression failed: {message}")]
    CompressFailed { message: String },
    #[error("decompression failed: {message}")]
    DecompressFailed { message: String },
    #[error("invalid input: {message}")]
    InvalidInput { message: String },
}

impl CompressError {
    pub fn compress(message: impl Into<String>) -> Self {
        Self::CompressFailed {
            message: message.into(),
        }
    }

    pub fn decompress(message: impl Into<String>) -> Self {
        Self::DecompressFailed {
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }
}
