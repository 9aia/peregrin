use std::fmt;

use ac_runtime_values::Error as VmError;

#[derive(Debug, Clone)]
pub enum IntlError {
    InvalidLocale { input: String },
    InvalidNumber { input: String },
    InvalidArgument { message: String },
    InvalidDate { input: String },
    InvalidCurrency { code: String },
    InvalidUnit { unit: String },
    InvalidType { type_name: String, valid: String },
    Formatting { message: String },
    DataProvider { message: String },
}

impl fmt::Display for IntlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale { input } => write!(f, "invalid locale: {input}"),
            Self::InvalidNumber { input } => write!(f, "invalid number: {input}"),
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
            Self::InvalidDate { input } => write!(f, "invalid date: {input}"),
            Self::InvalidCurrency { code } => write!(f, "invalid currency code: {code}"),
            Self::InvalidUnit { unit } => write!(f, "invalid unit: {unit}"),
            Self::InvalidType { type_name, valid } => {
                write!(f, "invalid type: {type_name} (expected: {valid})")
            }
            Self::Formatting { message } => write!(f, "formatting error: {message}"),
            Self::DataProvider { message } => write!(f, "data provider error: {message}"),
        }
    }
}

impl std::error::Error for IntlError {}

impl IntlError {
    pub fn into_vm_error(self) -> VmError {
        VmError::PrimitiveError(self.to_string())
    }
}

impl From<IntlError> for VmError {
    fn from(err: IntlError) -> Self {
        err.into_vm_error()
    }
}