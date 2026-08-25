use handl_vm_values_json::{Error, NonFunctionValue};

pub(crate) use crate::shared::{b, num};

pub(super) fn require_number(arg: &NonFunctionValue) -> Result<f64, Error> {
    arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })
}

pub(super) fn require_two_numbers(args: &[NonFunctionValue]) -> Result<(f64, f64), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_number(&args[0])?;
    let b = require_number(&args[1])?;
    Ok((a, b))
}

macro_rules! unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl handl_vm::Function<handl_vm_values_json::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[handl_vm_values_json::NonFunctionValue],
                _ctx: &mut dyn handl_vm::Context<handl_vm_values_json::NonFunctionValue>,
            ) -> Result<handl_vm_values_json::NonFunctionValue, handl_vm_values_json::Error> {
                let arg = args
                    .first()
                    .ok_or(handl_vm_values_json::Error::ArgumentCount {
                        expected: 1,
                        got: 0,
                    })?;
                let x = require_number(arg)?;
                Ok(num($body(x)))
            }
        }
    };
}

pub(super) use unary_fn;

macro_rules! bool_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl handl_vm::Function<handl_vm_values_json::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[handl_vm_values_json::NonFunctionValue],
                _ctx: &mut dyn handl_vm::Context<handl_vm_values_json::NonFunctionValue>,
            ) -> Result<handl_vm_values_json::NonFunctionValue, handl_vm_values_json::Error> {
                let arg = args
                    .first()
                    .ok_or(handl_vm_values_json::Error::ArgumentCount {
                        expected: 1,
                        got: 0,
                    })?;
                let x = require_number(arg)?;
                Ok(b($body(x)))
            }
        }
    };
}

pub(super) use bool_unary_fn;
