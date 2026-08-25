use handl_vm_values_json::Error;
use handl_vm_values_json::NonFunctionValue;
use serde_json::Value as JsonValue;

/// Extract raw bytes from a string or an array of numbers in the 0–255 range.
pub(crate) fn require_bytes(arg: &NonFunctionValue) -> Result<Vec<u8>, Error> {
    if let Some(s) = arg.as_str() {
        return Ok(s.as_bytes().to_vec());
    }

    if let Some(arr) = arg.as_array() {
        let mut bytes = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            let n = item.as_f64().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })?;
            if !n.is_finite() || n < 0.0 || n > 255.0 {
                return Err(Error::PrimitiveError(format!(
                    "byte at index {i} is not in the range 0-255: {n}"
                )));
            }
            bytes.push(n as u8);
        }
        return Ok(bytes);
    }

    Err(Error::TypeMismatch {
        expected: "string or array of numbers",
        got: "non-string non-array",
    })
}

pub(crate) fn require_one_bytes(args: &[NonFunctionValue]) -> Result<Vec<u8>, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_bytes(arg)
}

pub(crate) fn require_two_bytes(args: &[NonFunctionValue]) -> Result<(Vec<u8>, Vec<u8>), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_bytes(&args[0])?;
    let b = require_bytes(&args[1])?;
    Ok((a, b))
}

pub(crate) fn require_byte_value(args: &[NonFunctionValue], idx: usize) -> Result<u8, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    let n = arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })?;
    if !n.is_finite() || n < 0.0 || n > 255.0 {
        return Err(Error::PrimitiveError(format!(
            "byte value is not in the range 0-255: {n}"
        )));
    }
    Ok(n as u8)
}

pub(crate) fn require_index(args: &[NonFunctionValue], idx: usize) -> Result<usize, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    let n = arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })?;
    if n < 0.0 {
        return Err(Error::TypeMismatch {
            expected: "non-negative number",
            got: "negative number",
        });
    }
    Ok(n as usize)
}

pub(crate) fn byte_array(bytes: &[u8]) -> NonFunctionValue {
    let arr: Vec<JsonValue> = bytes
        .iter()
        .map(|b| JsonValue::Number(serde_json::Number::from(*b)))
        .collect();
    NonFunctionValue::new(JsonValue::Array(arr))
}

pub(crate) fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

pub(crate) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

pub(crate) fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}
