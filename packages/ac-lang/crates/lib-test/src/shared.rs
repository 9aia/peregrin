use ac_runtime::Context;
use ac_runtime_values::{Error, FunctionValue, NonFunctionValue};

use crate::error::TestError;

pub(crate) fn require_len(
    args: &[NonFunctionValue],
    expected: usize,
) -> Result<(), Error> {
    if args.len() != expected {
        return Err(TestError::ArgumentCount {
            expected,
            got: args.len(),
        }
        .into());
    }
    Ok(())
}

pub(crate) fn require_number(value: &NonFunctionValue) -> Result<f64, Error> {
    value
        .as_number()
        .ok_or_else(|| TestError::TypeMismatch {
            expected: "number",
            got: type_name(value),
        }
        .into())
}

pub(crate) fn require_integer(value: &NonFunctionValue) -> Result<i64, Error> {
    let n = require_number(value)?;
    if n.fract() != 0.0 {
        return Err(TestError::TypeMismatch {
            expected: "integer",
            got: type_name(value),
        }
        .into());
    }
    Ok(n as i64)
}

pub(crate) fn require_bool(value: &NonFunctionValue) -> Result<bool, Error> {
    value
        .as_bool()
        .ok_or_else(|| TestError::TypeMismatch {
            expected: "boolean",
            got: type_name(value),
        }
        .into())
}

pub(crate) fn require_string(value: &NonFunctionValue) -> Result<&str, Error> {
    value
        .as_str()
        .ok_or_else(|| TestError::TypeMismatch {
            expected: "string",
            got: type_name(value),
        }
        .into())
}

pub(crate) fn require_array(value: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    value
        .as_array()
        .ok_or_else(|| TestError::TypeMismatch {
            expected: "array",
            got: type_name(value),
        }
        .into())
}

pub(crate) fn require_function(value: &NonFunctionValue) -> Result<FunctionValue, Error> {
    FunctionValue::from_json(value.inner())
        .ok_or_else(|| {
            TestError::TypeMismatch {
                expected: "function",
                got: type_name(value),
            }
            .into()
        })
}

pub(crate) fn type_name(value: &NonFunctionValue) -> String {
    if value.is_null() {
        "null".to_string()
    } else if value.is_bool() {
        "boolean".to_string()
    } else if value.is_number() {
        "number".to_string()
    } else if value.is_string() {
        "string".to_string()
    } else if value.is_array() {
        "array".to_string()
    } else if value.is_object() {
        "object".to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn call_function(
    ctx: &mut dyn Context<NonFunctionValue>,
    func: &FunctionValue,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    if args.len() != func.params.len() {
        return Err(TestError::ArgumentCount {
            expected: func.params.len(),
            got: args.len(),
        }
        .into());
    }

    let vars: Vec<(String, NonFunctionValue)> = func
        .params
        .iter()
        .zip(args.iter())
        .map(|((name, _), value)| (name.clone(), value.clone()))
        .collect();

    ctx.eval_expr(&func.body, &vars)
}

pub(crate) fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

pub(crate) fn str(v: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(v.into())
}

pub(crate) fn obj(values: &[(&str, serde_json::Value)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in values {
        map.insert((*k).to_string(), v.clone());
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

