use ac_runtime_values::Error;

use ac_runtime_values::NonFunctionValue;


pub(super) fn require_string(arg: &NonFunctionValue) -> Result<&str, Error> {
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

pub(super) fn require_one_string(args: &[NonFunctionValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_string(arg)
}

pub(super) fn require_two_strings(args: &[NonFunctionValue]) -> Result<(&str, &str), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_string(&args[0])?;
    let b = require_string(&args[1])?;
    Ok((a, b))
}

pub(super) fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

pub(super) fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

pub(super) fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

macro_rules! string_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl ac_runtime::Function<ac_runtime_values::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[ac_runtime_values::NonFunctionValue],
                _ctx: &mut dyn ac_runtime::Context<ac_runtime_values::NonFunctionValue>,
            ) -> Result<ac_runtime_values::NonFunctionValue, ac_runtime_values::Error> {
                let input = $crate::r#str::common::require_one_string(args)?;
                Ok($crate::r#str::common::s($body(input)))
            }
        }
    };
}
pub(super) use string_unary_fn;

macro_rules! string_predicate_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl ac_runtime::Function<ac_runtime_values::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[ac_runtime_values::NonFunctionValue],
                _ctx: &mut dyn ac_runtime::Context<ac_runtime_values::NonFunctionValue>,
            ) -> Result<ac_runtime_values::NonFunctionValue, ac_runtime_values::Error> {
                let (input, pattern) = $crate::r#str::common::require_two_strings(args)?;
                Ok($crate::r#str::common::b($body(input, pattern)))
            }
        }
    };
}
pub(super) use string_predicate_fn;
