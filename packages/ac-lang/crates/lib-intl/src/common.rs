use ac_runtime::NonFunctionValue;
use ac_vm::Error;
use fixed_decimal::{Decimal, FloatPrecision};
use icu::locale::Locale;
use icu_provider_blob::BlobDataProvider;

use super::error::IntlError;
use super::provider::IntlProvider;

/// A data provider that is either compiled (static) or blob-backed (runtime).
pub(super) enum DataProvider {
    Compiled,
    Blob(BlobDataProvider),
}

impl DataProvider {
    pub fn as_buffer_provider(&self) -> Option<&BlobDataProvider> {
        match self {
            DataProvider::Blob(p) => Some(p),
            _ => None,
        }
    }
}

/// Create a `DataProvider` from an optional `IntlProvider`.
pub(super) fn make_provider(provider: &Option<IntlProvider>) -> Result<DataProvider, Error> {
    match provider {
        Some(p) => {
            let bdp = p.as_blob_data_provider()
                .map_err(|e| IntlError::DataProvider { message: e.to_string() }.into_vm_error())?;
            Ok(DataProvider::Blob(bdp))
        }
        None => Ok(DataProvider::Compiled),
    }
}

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
pub(super) fn optional_object(
    args: &[NonFunctionValue],
    idx: usize,
) -> Result<Option<&serde_json::Map<String, serde_json::Value>>, Error> {
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

/// Get an integer field from an options object.
/// Accepts both JSON numbers and numeric strings.
pub(super) fn opt_int(
    obj: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<i64> {
    obj.and_then(|o| o.get(key)).and_then(|v| {
        v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

/// Get a boolean field from an options object.
/// Accepts both JSON booleans and string "true"/"false".
pub(super) fn opt_bool(
    obj: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<bool> {
    obj.and_then(|o| o.get(key)).and_then(|v| {
        v.as_bool().or_else(|| v.as_str().and_then(|s| match s {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }))
    })
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
