use std::fmt;

use ac_runtime::Error as VmError;

#[derive(Debug, Clone)]
pub enum RandError {
    InvalidRange { min: f64, max: f64 },
    EmptyArray,
    InvalidSampleCount { requested: usize, available: usize },
    WeightsLengthMismatch { items: usize, weights: usize },
    NonPositiveWeights,
    NonNumericWeight { index: usize },
    InvalidStringLength { length: i64 },
    InvalidByteLength { length: i64 },
    InvalidCharset { name: String },
}

impl fmt::Display for RandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange { min, max } => {
                write!(f, "invalid range: min ({min}) must be <= max ({max})")
            }
            Self::EmptyArray => write!(f, "cannot pick from an empty array"),
            Self::InvalidSampleCount {
                requested,
                available,
            } => {
                write!(
                    f,
                    "cannot sample {requested} elements from array of {available}"
                )
            }
            Self::WeightsLengthMismatch { items, weights } => {
                write!(
                    f,
                    "items array length ({items}) must match weights array length ({weights})"
                )
            }
            Self::NonPositiveWeights => {
                write!(f, "weights must contain at least one positive value")
            }
            Self::NonNumericWeight { index } => {
                write!(f, "weight at index {index} is not a number")
            }
            Self::InvalidStringLength { length } => {
                write!(f, "string length must be non-negative, got {length}")
            }
            Self::InvalidByteLength { length } => {
                write!(f, "byte length must be non-negative, got {length}")
            }
            Self::InvalidCharset { name } => {
                write!(
                    f,
                    "unknown charset '{name}' — valid options: alphanumeric, alpha, numeric, hex, lower, upper"
                )
            }
        }
    }
}

impl std::error::Error for RandError {}

impl From<RandError> for VmError {
    fn from(err: RandError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl RandError {
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}
