use ac_runtime::NonFunctionValue;
use ac_vm::Error;

use crate::locale::Locale;

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
/// Defaults to `Locale::en()` if not present.
pub(super) fn optional_locale(args: &[NonFunctionValue], idx: usize) -> Result<Locale, Error> {
    match args.get(idx) {
        Some(arg) => {
            let s = arg.as_str().ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string",
            })?;
            Locale::parse(s).map_err(Into::into)
        }
        None => Ok(Locale::en()),
    }
}

pub(super) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}
