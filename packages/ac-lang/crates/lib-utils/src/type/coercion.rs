use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0})
}

fn number_to_string(n: f64) -> String {
    format!("{}", n)
}

#[derive(Clone, Copy)]
pub struct ToString;
impl Function for ToString {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let result = if let Some(s) = arg.as_str() {
            s.to_string()
        } else if let Some(n) = arg.as_number() {
            number_to_string(n)
        } else if let Some(b) = arg.as_bool() {
            b.to_string()
        } else if arg.is_null() {
            "null".to_string()
        } else {
            arg.inner().to_string()
        };
        Ok(NonFunctionValue::new(result))
    }
}

#[derive(Clone, Copy)]
pub struct ToNumber;
impl Function for ToNumber {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let result = if let Some(n) = arg.as_number() {
            n
        } else if let Some(b) = arg.as_bool() {
            if b { 1.0 } else { 0.0 }
        } else if let Some(s) = arg.as_str() {
            let trimmed = s.trim();
            match trimmed.parse::<f64>() {
                Ok(parsed) if parsed.is_finite() => parsed,
                _ => 0.0}
        } else if arg.is_null() {
            0.0
        } else {
            0.0
        };
        Ok(NonFunctionValue::new(serde_json::json!(result)))
    }
}

#[derive(Clone, Copy)]
pub struct ToBool;
impl Function for ToBool {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let result = if let Some(b) = arg.as_bool() {
            b
        } else if let Some(n) = arg.as_number() {
            n != 0.0
        } else if let Some(s) = arg.as_str() {
            !s.is_empty()
        } else if arg.is_null() {
            false
        } else if let Some(arr) = arg.as_array() {
            !arr.is_empty()
        } else if let Some(obj) = arg.as_object() {
            !obj.is_empty()
        } else {
            false
        };
        Ok(NonFunctionValue::new(result))
    }
}
