use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use serde_json::Value as JsonValue;

fn resolve_path(value: &NonFunctionValue, path: &[&str]) -> Result<NonFunctionValue, Error> {
    let mut current = value.inner().clone();
    for segment in path {
        current = match &current {
            serde_json::Value::Object(obj) => obj
                .get(*segment)
                .cloned()
                .ok_or_else(|| Error::UnknownVariable(format!("{segment}")))?,
            serde_json::Value::Array(arr) => segment
                .parse::<usize>()
                .ok()
                .and_then(|i| arr.get(i).cloned())
                .ok_or_else(|| Error::UnknownVariable(format!("{segment}")))?,
            _ => {
                return Err(Error::UnknownVariable(format!("{segment}")));
            }
        };
    }
    Ok(NonFunctionValue::new(current))
}

fn require_one_string(args: &[NonFunctionValue]) -> Result<String, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    arg.as_str()
        .map(|s| s.to_string())
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn require_path(args: &[NonFunctionValue]) -> Result<Vec<&str>, Error> {
    let arg = args.get(1).ok_or(Error::ArgumentCount {
        expected: 2,
        got: args.len(),
    })?;
    let arr = arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })?;
    arr.iter()
        .map(|v| {
            v.as_str().ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string",
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

#[derive(Clone)]
pub struct Add;
impl Function for Add {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(serde_json::json!(sum)))
    }
}

#[derive(Clone)]
pub struct Mul;
impl Function for Mul {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let product = args.iter().filter_map(|v| v.as_number()).product::<f64>();
        Ok(NonFunctionValue::new(serde_json::json!(product)))
    }
}

#[derive(Clone)]
pub struct Concat;
impl Function for Concat {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let mut result = String::new();
        for arg in args {
            match arg.inner() {
                serde_json::Value::String(s) => result.push_str(s),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: "string",
                        got: "non-string",
                    });
                }
            }
        }
        Ok(NonFunctionValue::new(result))
    }
}

#[derive(Clone)]
pub struct Len;
impl Function for Len {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let len = match arg.inner() {
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(a) => a.len(),
            serde_json::Value::Object(o) => o.len(),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "string, array, or object",
                    got: "other",
                });
            }
        };
        Ok(NonFunctionValue::new(serde_json::json!(len)))
    }
}

#[derive(Clone)]
pub struct Var;
impl Function for Var {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let name = require_one_string(args)?;
        match ctx.get(&name) {
            Ok(value) => Ok(value),
            Err(_) => {
                let doc = ctx.get("doc")?;
                resolve_path(&doc, &[&name])
            }
        }
    }
}

#[derive(Clone)]
pub struct Resolve;
impl Function for Resolve {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let value = args.first().ok_or(Error::ArgumentCount {
            expected: 2,
            got: 0,
        })?;
        let path = require_path(args)?;
        resolve_path(value, &path)
    }
}

#[derive(Clone)]
pub struct Apply;
impl Function for Apply {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let fn_value = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let fn_val = fn_value.as_function().ok_or(Error::TypeMismatch {
            expected: "function",
            got: "non-function",
        })?;

        let fn_args = &args[1..];
        let vars: Vec<(String, NonFunctionValue)> = fn_val
            .params
            .iter()
            .zip(fn_args.iter())
            .map(|((pname, _), arg)| (pname.clone(), arg.clone()))
            .collect();

        ctx.eval_expr(&fn_val.body, &vars)
    }
}

#[derive(Clone)]
pub struct Array;
impl Function for Array {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arr: Vec<JsonValue> = args.iter().map(|v| v.inner().clone()).collect();
        Ok(NonFunctionValue::new(JsonValue::Array(arr)))
    }
}
