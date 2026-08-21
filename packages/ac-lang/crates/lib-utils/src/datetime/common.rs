use ac_runtime::NonFunctionValue;
use ac_vm::Error;
use chrono::{DateTime, Utc};

pub(super) fn require_string(arg: &NonFunctionValue) -> Result<&str, Error> {
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

pub(super) fn require_number(arg: &NonFunctionValue) -> Result<f64, Error> {
    arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })
}

pub(super) fn require_one_string(args: &[NonFunctionValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_string(arg)
}

pub(super) fn require_two_strings(args: &[NonFunctionValue]) -> Result<(&str, &str), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_string(&args[0])?;
    let b = require_string(&args[1])?;
    Ok((a, b))
}

pub(super) fn require_string_and_number(args: &[NonFunctionValue]) -> Result<(&str, f64), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let s = require_string(&args[0])?;
    let n = require_number(&args[1])?;
    Ok((s, n))
}

pub(super) fn require_three_strings(args: &[NonFunctionValue]) -> Result<(&str, &str, &str), Error> {
    if args.len() < 3 {
        return Err(Error::ArgumentCount {
            expected: 3,
            got: args.len(),
        });
    }
    let a = require_string(&args[0])?;
    let b = require_string(&args[1])?;
    let c = require_string(&args[2])?;
    Ok((a, b, c))
}

pub(super) fn parse_iso(s: &str) -> Result<DateTime<Utc>, Error> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        })
        .map_err(|_| Error::PrimitiveError(format!("invalid datetime: {s}")))
}

pub(super) fn parse_with_format(s: &str, fmt: &str) -> Result<DateTime<Utc>, Error> {
    chrono::NaiveDateTime::parse_from_str(s, fmt)
        .map(|ndt| ndt.and_utc())
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, fmt)
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        })
        .map_err(|_| Error::PrimitiveError(format!("parse failed: {s} with format {fmt}")))
}

pub(super) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

pub(super) fn n(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

pub(super) fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}
