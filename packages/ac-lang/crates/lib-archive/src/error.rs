use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ArchiveError {
    #[error("pack failed: {message}")]
    PackFailed { message: String },
    #[error("unpack failed: {message}")]
    UnpackFailed { message: String },
    #[error("invalid entry path: {message}")]
    InvalidPath { message: String },
}

impl ArchiveError {
    pub fn pack(message: impl Into<String>) -> Self {
        Self::PackFailed {
            message: message.into(),
        }
    }

    pub fn unpack(message: impl Into<String>) -> Self {
        Self::UnpackFailed {
            message: message.into(),
        }
    }

    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::InvalidPath {
            message: message.into(),
        }
    }
}
