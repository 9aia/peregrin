use ac_runtime::NonFunctionValue;
use ac_runtime::Error;

/// Require a number argument at `idx`.
pub(super) fn require_number(args: &[NonFunctionValue], idx: usize) -> Result<f64, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len()})?;
    arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number"})
}

/// Require an integer argument at `idx`.
pub(super) fn require_int(args: &[NonFunctionValue], idx: usize) -> Result<i64, Error> {
    let n = require_number(args, idx)?;
    if !n.is_finite() || n.fract() != 0.0 {
        return Err(Error::TypeMismatch {
            expected: "integer",
            got: "non-integer number"});
    }
    Ok(n as i64)
}

/// Require an array argument at `idx`.
pub(super) fn require_array(
    args: &[NonFunctionValue],
    idx: usize,
) -> Result<&[serde_json::Value], Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len()})?;
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array"})
}

pub(super) fn num(value: impl Into<f64>) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value.into()))
}

pub(super) fn bool_val(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

pub(super) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

pub(super) fn array(values: Vec<serde_json::Value>) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(values))
}
