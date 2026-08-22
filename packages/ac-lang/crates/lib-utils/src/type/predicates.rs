use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn s(value: &str) -> NonFunctionValue {
    NonFunctionValue::new(value.to_string())
}

macro_rules! type_predicate {
    ($struct_name:ident, $predicate:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut Context,
            ) -> Result<NonFunctionValue, Error> {
                let arg = require_one_arg(args)?;
                Ok(b($predicate(arg)))
            }
        }
    };
}

type_predicate!(IsNull, |arg: &NonFunctionValue| arg.is_null());
type_predicate!(IsBool, |arg: &NonFunctionValue| arg.is_bool());
type_predicate!(IsNumber, |arg: &NonFunctionValue| arg.is_number());
type_predicate!(IsString, |arg: &NonFunctionValue| arg.is_string());
type_predicate!(IsArray, |arg: &NonFunctionValue| arg.is_array());
type_predicate!(IsObject, |arg: &NonFunctionValue| arg.is_object());

#[derive(Clone, Copy)]
pub struct TypeOf;
impl Function for TypeOf {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let type_name = if arg.is_null() {
            "null"
        } else if arg.is_bool() {
            "boolean"
        } else if arg.is_number() {
            "number"
        } else if arg.is_string() {
            "string"
        } else if arg.is_array() {
            "array"
        } else if arg.is_object() {
            "object"
        } else {
            "unknown"
        };
        Ok(s(type_name))
    }
}

#[derive(Clone, Copy)]
pub struct IsEmpty;
impl Function for IsEmpty {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let empty = if arg.is_null() {
            true
        } else if let Some(str_val) = arg.as_str() {
            str_val.is_empty()
        } else if let Some(arr) = arg.as_array() {
            arr.is_empty()
        } else if let Some(obj) = arg.as_object() {
            obj.is_empty()
        } else {
            false
        };
        Ok(b(empty))
    }
}

#[derive(Clone, Copy)]
pub struct IsTruthy;
impl Function for IsTruthy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let truthy = if let Some(b_val) = arg.as_bool() {
            b_val
        } else if let Some(n) = arg.as_number() {
            n != 0.0
        } else if let Some(str_val) = arg.as_str() {
            !str_val.is_empty()
        } else if arg.is_null() {
            false
        } else if let Some(arr) = arg.as_array() {
            !arr.is_empty()
        } else if let Some(obj) = arg.as_object() {
            !obj.is_empty()
        } else {
            false
        };
        Ok(b(truthy))
    }
}
