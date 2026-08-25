use handl_vm_values_json::Error;

use handl_vm_values_json::NonFunctionValue;

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
        impl handl_vm::Function<handl_vm_values_json::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[handl_vm_values_json::NonFunctionValue],
                _ctx: &mut dyn handl_vm::Context<handl_vm_values_json::NonFunctionValue>,
            ) -> Result<handl_vm_values_json::NonFunctionValue, handl_vm_values_json::Error> {
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
        impl handl_vm::Function<handl_vm_values_json::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[handl_vm_values_json::NonFunctionValue],
                _ctx: &mut dyn handl_vm::Context<handl_vm_values_json::NonFunctionValue>,
            ) -> Result<handl_vm_values_json::NonFunctionValue, handl_vm_values_json::Error> {
                let (input, pattern) = $crate::r#str::common::require_two_strings(args)?;
                Ok($crate::r#str::common::b($body(input, pattern)))
            }
        }
    };
}
pub(super) use string_predicate_fn;
