use ac_runtime::{Context, Function};
use ac_runtime_values::{Error, FunctionValue, NonFunctionValue};
use serde_json::{json, Value as JsonValue};

use crate::shared::{apply_fn, require_function_value};

fn as_result_tag(value: &JsonValue) -> Option<&str> {
    value.get("_tag")?.as_str()
}

fn require_payload<'a>(value: &'a JsonValue, tag: &str) -> Result<&'a JsonValue, Error> {
    let (field, expected, got) = if tag == "Ok" {
        (
            "value",
            "Ok result with value field",
            "Ok result without value field",
        )
    } else {
        (
            "error",
            "Err result with error field",
            "Err result without error field",
        )
    };
    value.get(field).ok_or_else(|| Error::TypeMismatch {
        expected,
        got,
    })
}

fn require_match_branches(
    args: &[NonFunctionValue],
) -> std::result::Result<(FunctionValue, FunctionValue), Error> {
    if args.len() < 3 {
        return std::result::Result::Err(Error::ArgumentCount {
            expected: 3,
            got: args.len(),
        });
    }
    let on_ok = require_function_value(&args[1])?;
    let on_err = require_function_value(&args[2])?;
    std::result::Result::Ok((on_ok, on_err))
}

#[derive(Clone, Copy)]
pub struct Ok;
impl Function<NonFunctionValue> for Ok {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        std::result::Result::Ok(NonFunctionValue::new(json!({
            "_tag": "Ok",
            "value": arg.inner().clone(),
        })))
    }
}

#[derive(Clone, Copy)]
pub struct Err;
impl Function<NonFunctionValue> for Err {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        std::result::Result::Ok(NonFunctionValue::new(json!({
            "_tag": "Err",
            "error": arg.inner().clone(),
        })))
    }
}

#[derive(Clone, Copy)]
pub struct IsOk;
impl Function<NonFunctionValue> for IsOk {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let is_ok = as_result_tag(arg.inner()) == Some("Ok");
        std::result::Result::Ok(NonFunctionValue::new(json!(is_ok)))
    }
}

#[derive(Clone, Copy)]
pub struct IsErr;
impl Function<NonFunctionValue> for IsErr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let is_err = as_result_tag(arg.inner()) == Some("Err");
        std::result::Result::Ok(NonFunctionValue::new(json!(is_err)))
    }
}

#[derive(Clone, Copy)]
pub struct Match;
impl Function<NonFunctionValue> for Match {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return std::result::Result::Err(Error::ArgumentCount {
                expected: 3,
                got: 0,
            });
        }
        let (on_ok, on_err) = require_match_branches(args)?;
        let inner = args[0].inner();
        let tag = as_result_tag(inner).ok_or(Error::TypeMismatch {
            expected: "Ok or Err",
            got: "non-result",
        })?;
        let payload = match tag {
            "Ok" => require_payload(inner, "Ok")?,
            "Err" => require_payload(inner, "Err")?,
            _ => {
                return std::result::Result::Err(Error::PrimitiveError(format!(
                    "unknown result tag: {tag}"
                )));
            }
        };
        let branch = if tag == "Ok" { &on_ok } else { &on_err };
        apply_fn(ctx, branch, payload)
    }
}
