use ac_runtime::{Context, Function};
use ac_runtime_values::{Error, FunctionValue, NonFunctionValue};

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
pub struct Apply;
impl Function<NonFunctionValue> for Apply {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let fn_value = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let fn_val = FunctionValue::from_json(fn_value.inner()).ok_or(Error::TypeMismatch {
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
pub struct Var;
impl Function<NonFunctionValue> for Var {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
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
impl Function<NonFunctionValue> for Resolve {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let value = args.first().ok_or(Error::ArgumentCount {
            expected: 2,
            got: 0,
        })?;
        let path = require_path(args)?;
        resolve_path(value, &path)
    }
}
