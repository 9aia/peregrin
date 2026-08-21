use std::fmt;

use ac_vm::Error as VmError;

#[derive(Debug, Clone)]
pub enum IntlError {
    InvalidLocale { input: String },
    UnsupportedLocale { locale: String },
    InvalidNumber { input: String },
    InvalidArgument { message: String },
}

impl fmt::Display for IntlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale { input } => write!(f, "invalid locale: {input}"),
            Self::UnsupportedLocale { locale } => write!(f, "unsupported locale: {locale}"),
            Self::InvalidNumber { input } => write!(f, "invalid number: {input}"),
            Self::InvalidArgument { message } => write!(f, "invalid argument: {message}"),
        }
    }
}

impl std::error::Error for IntlError {}

impl From<IntlError> for VmError {
    fn from(err: IntlError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl IntlError {
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}
