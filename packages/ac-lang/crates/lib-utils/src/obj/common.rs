use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime::Error;

pub(super) fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0})
}

pub(super) fn require_two_args(
    args: &[NonFunctionValue],
) -> Result<(&NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len()});
    }
    Ok((&args[0], &args[1]))
}

pub(super) fn require_three_args(
    args: &[NonFunctionValue],
) -> Result<(&NonFunctionValue, &NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 3 {
        return Err(Error::ArgumentCount {
            expected: 3,
            got: args.len()});
    }
    Ok((&args[0], &args[1], &args[2]))
}

pub(super) fn require_object(
    arg: &NonFunctionValue,
) -> Result<&serde_json::Map<String, serde_json::Value>, Error> {
    arg.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object"})
}

pub(super) fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array"})
}

pub(super) fn require_string(arg: &NonFunctionValue) -> Result<String, Error> {
    arg.as_str()
        .map(|s| s.to_string())
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string"})
}

pub(super) fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function"})
}

pub(super) fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

pub(super) fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

pub(super) fn obj_value(map: serde_json::Map<String, serde_json::Value>) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Object(map))
}

pub(super) fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

pub(super) fn bind_param(
    fn_val: &FunctionValue,
    value: &serde_json::Value,
) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| vec![(name.clone(), NonFunctionValue::new(value.clone()))])
        .unwrap_or_default()
}

pub(super) fn is_truthy_value(value: &NonFunctionValue) -> bool {
    if let Some(b) = value.as_bool() {
        b
    } else if let Some(n) = value.as_number() {
        n != 0.0
    } else if let Some(s) = value.as_str() {
        !s.is_empty()
    } else if value.is_null() {
        false
    } else if let Some(arr) = value.as_array() {
        !arr.is_empty()
    } else if let Some(obj) = value.as_object() {
        !obj.is_empty()
    } else {
        true
    }
}

pub(super) fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == f.trunc() {
                    format!("{f:.0}")
                } else {
                    f.to_string()
                }
            } else {
                n.to_string()
            }
        }
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "unknown".to_string())}
}

pub(super) fn deep_merge_values(a: &serde_json::Value, b: &serde_json::Value) -> serde_json::Value {
    match (a, b) {
        (serde_json::Value::Object(a_obj), serde_json::Value::Object(b_obj)) => {
            let mut result = a_obj.clone();
            for (k, v) in b_obj {
                if let Some(existing) = result.get(k) {
                    result.insert(k.clone(), deep_merge_values(existing, v));
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
            serde_json::Value::Object(result)
        }
        (_, b) => b.clone()}
}

pub(super) fn set_path_recursive(
    value: serde_json::Value,
    segments: &[&str],
    new_value: serde_json::Value,
) -> serde_json::Value {
    if segments.is_empty() {
        return new_value;
    }
    let segment = segments[0];
    let rest = &segments[1..];
    let mut map = match value {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new()};
    let current = map.get(segment).cloned().unwrap_or(serde_json::Value::Null);
    let updated = set_path_recursive(current, rest, new_value);
    map.insert(segment.to_string(), updated);
    serde_json::Value::Object(map)
}

pub(super) fn update_path_recursive(
    value: serde_json::Value,
    segments: &[&str],
    fn_val: &FunctionValue,
    ctx: &mut dyn ac_runtime::Context,
) -> Result<serde_json::Value, Error> {
    if segments.is_empty() {
        let vars = bind_param(fn_val, &value);
        let result = ctx.eval_expr(&fn_val.body, &vars)?;
        return Ok(result.into_inner());
    }
    let segment = segments[0];
    let rest = &segments[1..];
    let mut map = match value {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new()};
    let current = map.get(segment).cloned().unwrap_or(serde_json::Value::Null);
    let updated = update_path_recursive(current, rest, fn_val, ctx)?;
    map.insert(segment.to_string(), updated);
    Ok(serde_json::Value::Object(map))
}
