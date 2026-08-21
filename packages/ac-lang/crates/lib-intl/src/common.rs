use ac_runtime::NonFunctionValue;
use ac_vm::Error;
use fixed_decimal::{Decimal, FloatPrecision};
use icu::locale::Locale;

use super::error::IntlError;

/// Require a number argument at `idx`.
pub(super) fn require_number(args: &[NonFunctionValue], idx: usize) -> Result<f64, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })
}

/// Require a string argument at `idx`.
pub(super) fn require_string(args: &[NonFunctionValue], idx: usize) -> Result<&str, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

/// Require an array argument at `idx`.
pub(super) fn require_array(args: &[NonFunctionValue], idx: usize) -> Result<&[serde_json::Value], Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

/// Require an object argument at `idx`.
pub(super) fn require_object(args: &[NonFunctionValue], idx: usize) -> Result<&serde_json::Map<String, serde_json::Value>, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object",
    })
}

/// Extract an optional locale argument at the given index.
/// Defaults to `en` if not present.
pub(super) fn optional_locale(args: &[NonFunctionValue], idx: usize) -> Result<Locale, Error> {
    match args.get(idx) {
        Some(arg) => {
            let s = arg.as_str().ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string",
            })?;
            Locale::try_from_str(s).map_err(|_| {
                IntlError::InvalidLocale { input: s.to_string() }.into_vm_error()
            })
        }
        None => Ok(Locale::try_from_str("en").unwrap())
    }
}

/// Extract an optional options object at the given index.
pub(super) fn optional_object<'a>(
    args: &'a [NonFunctionValue],
    idx: usize,
) -> Result<Option<&'a serde_json::Map<String, serde_json::Value>>, Error> {
    match args.get(idx) {
        Some(arg) => {
            let obj = arg.as_object().ok_or(Error::TypeMismatch {
                expected: "object",
                got: "non-object",
            })?;
            Ok(Some(obj))
        }
        None => Ok(None),
    }
}

/// Get a string field from an options object.
pub(super) fn opt_str<'a>(
    obj: Option<&'a serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<&'a str> {
    obj.and_then(|o| o.get(key)).and_then(|v| v.as_str())
}

/// Get a number field from an options object.
pub(super) fn opt_num(
    obj: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<f64> {
    obj.and_then(|o| o.get(key)).and_then(|v| v.as_f64())
}

/// Get an integer field from an options object.
pub(super) fn opt_int(
    obj: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<i64> {
    obj.and_then(|o| o.get(key)).and_then(|v| v.as_i64())
}

/// Get a boolean field from an options object.
pub(super) fn opt_bool(
    obj: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<bool> {
    obj.and_then(|o| o.get(key)).and_then(|v| v.as_bool())
}

pub(super) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

/// Convert an f64 into an ICU4X `Decimal` using `FloatPrecision::RoundTrip`
/// for exact round-trip representation (avoids `0.1 + 0.2` → `"0.30000000000000004"`).
pub(super) fn decimal_from_f64(n: f64) -> Decimal {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return Decimal::from(0_i64);
    }
    Decimal::try_from_f64(n, FloatPrecision::RoundTrip)
        .unwrap_or(Decimal::from(0_i64))
}

/// Convert an f64 into a `fixed_decimal::Decimal` (the unsigned variant)
/// using `FloatPrecision::RoundTrip`.
pub(super) fn fixed_decimal_from_f64(n: f64) -> fixed_decimal::UnsignedDecimal {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return fixed_decimal::UnsignedDecimal::from(0u32);
    }
    fixed_decimal::UnsignedDecimal::try_from_f64(n.abs(), FloatPrecision::RoundTrip)
        .unwrap_or(fixed_decimal::UnsignedDecimal::from(0u32))
}
