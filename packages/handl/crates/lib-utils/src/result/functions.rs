use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};
use serde_json::{Value as JsonValue, json};

fn as_result_tag(value: &JsonValue) -> Option<&str> {
    value.get("_tag")?.as_str()
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
