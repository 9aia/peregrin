use ac_runtime_values::Error as VmError;
use std::fmt;

#[derive(Debug, Clone)]
pub enum CollectionsError {
    Empty,
    OutOfBounds { index: i64, length: usize },
}

impl fmt::Display for CollectionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "collection is empty"),
            Self::OutOfBounds { index, length } => {
                write!(f, "index {index} is out of bounds for collection of length {length}")
            }
        }
    }
}

impl std::error::Error for CollectionsError {}

impl From<CollectionsError> for VmError {
    fn from(err: CollectionsError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}
