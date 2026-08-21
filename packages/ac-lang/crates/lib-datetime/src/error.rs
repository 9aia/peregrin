use std::fmt;

use ac_vm::Error as VmError;

#[derive(Debug, Clone)]
pub enum DatetimeError {
    InvalidDatetime { input: String },
    InvalidUnixTimestamp { value: f64 },
    InvalidTimezoneOffset { offset: String },
    InvalidDateComponents {
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    },
    YearOverflow { year: i64 },
    InvalidDateAfterArithmetic {
        year: i64,
        month: i64,
        day: u32,
    },
    ParseFailed { input: String, format: String },
}

impl fmt::Display for DatetimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDatetime { input } => {
                write!(f, "invalid datetime: {input}")
            }
            Self::InvalidUnixTimestamp { value } => {
                write!(f, "invalid unix timestamp: {value}")
            }
            Self::InvalidTimezoneOffset { offset } => {
                write!(f, "invalid timezone offset: {offset}")
            }
            Self::InvalidDateComponents {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => {
                write!(
                    f,
                    "invalid date components: {year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"
                )
            }
            Self::YearOverflow { year } => {
                write!(f, "year overflow: {year}")
            }
            Self::InvalidDateAfterArithmetic { year, month, day } => {
                write!(f, "invalid date after month arithmetic: {year}-{month}-{day}")
            }
            Self::ParseFailed { input, format } => {
                write!(f, "parse failed: {input} with format {format}")
            }
        }
    }
}

impl std::error::Error for DatetimeError {}

impl From<DatetimeError> for VmError {
    fn from(err: DatetimeError) -> Self {
        VmError::PrimitiveError(err.to_string())
    }
}

impl DatetimeError {
    pub fn into_vm_error(self) -> VmError {
        self.into()
    }
}
