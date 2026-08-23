use ac_runtime::NonFunctionValue;
use ac_vm::Error;

use crate::error::CryptoError;

/// Require a string argument at `idx`.
pub(super) fn require_string(args: &[NonFunctionValue], idx: usize) -> Result<String, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len()})?;
    arg.as_str()
        .map(|s| s.to_string())
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string"})
}

/// Extract bytes from an argument that is either a string (UTF-8) or an
/// array of integers in the range 0–255.
pub(super) fn extract_bytes(args: &[NonFunctionValue], idx: usize) -> Result<Vec<u8>, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len()})?;

    if let Some(s) = arg.as_str() {
        return Ok(s.as_bytes().to_vec());
    }

    if let Some(arr) = arg.as_array() {
        let mut bytes = Vec::with_capacity(arr.len());
        for item in arr.iter() {
            let n = item.as_f64().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number"})?;
            if !n.is_finite() || n < 0.0 || n > 255.0 {
                return Err(CryptoError::InvalidByteValue { value: n }.into());
            }
            bytes.push(n as u8);
        }
        return Ok(bytes);
    }

    Err(Error::TypeMismatch {
        expected: "string or array of numbers",
        got: "non-string non-array"})
}

pub(super) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

pub(super) fn byte_array(bytes: &[u8]) -> NonFunctionValue {
    let arr: Vec<serde_json::Value> = bytes
        .iter()
        .map(|b| serde_json::json!(b))
        .collect();
    NonFunctionValue::new(serde_json::Value::Array(arr))
}
