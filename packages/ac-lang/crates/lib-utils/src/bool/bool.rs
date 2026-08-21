use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

/// The `bool` module — boolean utility functions.
///
/// Provides functions callable as `bool.and(a, b)`, `bool.or(a, b)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct BoolModule;

impl Module for BoolModule {
    fn name(&self) -> &str {
        "bool"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "and" => Some(&And),
            "or" => Some(&Or),
            "not" => Some(&Not),
            "xor" => Some(&Xor),
            "nand" => Some(&Nand),
            "nor" => Some(&Nor),
            "if" => Some(&If),
            "eq" => Some(&Eq),
            "from" => Some(&From),
            _ => None,
        }
    }
}

fn require_bool(arg: &RuntimeValue) -> Result<bool, Error> {
    arg.as_bool().ok_or(Error::TypeMismatch {
        expected: "boolean",
        got: "non-boolean",
    })
}

fn require_one_bool(args: &[RuntimeValue]) -> Result<bool, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_bool(arg)
}

fn require_two_bools(args: &[RuntimeValue]) -> Result<(bool, bool), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_bool(&args[0])?;
    let b = require_bool(&args[1])?;
    Ok((a, b))
}

fn to_bool(value: bool) -> RuntimeValue {
    RuntimeValue::new(value)
}

macro_rules! bool_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
                let input = require_one_bool(args)?;
                Ok(to_bool($body(input)))
            }
        }
    };
}

macro_rules! bool_binary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
                let (a, b) = require_two_bools(args)?;
                Ok(to_bool($body(a, b)))
            }
        }
    };
}

bool_unary_fn!(Not, |a: bool| !a);
bool_binary_fn!(And, |a: bool, b: bool| a && b);
bool_binary_fn!(Or, |a: bool, b: bool| a || b);
bool_binary_fn!(Xor, |a: bool, b: bool| a ^ b);
bool_binary_fn!(Nand, |a: bool, b: bool| !(a && b));
bool_binary_fn!(Nor, |a: bool, b: bool| !(a || b));
bool_binary_fn!(If, |a: bool, b: bool| !a || b);
bool_binary_fn!(Eq, |a: bool, b: bool| a == b);

#[derive(Clone, Copy)]
pub struct From;
impl Function for From {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        if let Some(b) = arg.as_bool() {
            return Ok(RuntimeValue::new(b));
        }
        if let Some(n) = arg.as_number() {
            return Ok(RuntimeValue::new(n != 0.0));
        }
        if let Some(s) = arg.as_str() {
            return Ok(RuntimeValue::new(!s.is_empty()));
        }
        if arg.is_null() {
            return Ok(RuntimeValue::new(false));
        }
        if let Some(arr) = arg.as_array() {
            return Ok(RuntimeValue::new(!arr.is_empty()));
        }
        if let Some(obj) = arg.as_object() {
            return Ok(RuntimeValue::new(!obj.is_empty()));
        }
        Ok(RuntimeValue::new(true))
    }
}
