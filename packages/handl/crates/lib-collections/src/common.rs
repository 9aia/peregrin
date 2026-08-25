use handl_vm_values_json::{Error, NonFunctionValue};

/// Require an argument at `idx` to be an array.
pub(super) fn require_array(
    args: &[NonFunctionValue],
    idx: usize,
) -> Result<&[serde_json::Value], Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

/// Require a single array argument.
pub(super) fn require_one_array(args: &[NonFunctionValue]) -> Result<&[serde_json::Value], Error> {
    if args.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    require_array(args, 0)
}

/// Require an argument at `idx` to be an object.
pub(super) fn require_object(
    args: &[NonFunctionValue],
    idx: usize,
) -> Result<&serde_json::Map<String, serde_json::Value>, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object",
    })
}

/// Require two array arguments.
pub(super) fn require_two_arrays(
    args: &[NonFunctionValue],
) -> Result<(&[serde_json::Value], &[serde_json::Value]), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    Ok((require_array(args, 0)?, require_array(args, 1)?))
}

/// Require a single object argument.
pub(super) fn require_one_object(
    args: &[NonFunctionValue],
) -> Result<&serde_json::Map<String, serde_json::Value>, Error> {
    if args.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    require_object(args, 0)
}

pub(super) fn num(value: impl Into<f64>) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value.into()))
}

pub(super) fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

pub(super) fn arr(values: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(values.to_vec()))
}

pub(super) fn obj(map: serde_json::Map<String, serde_json::Value>) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Object(map))
}
