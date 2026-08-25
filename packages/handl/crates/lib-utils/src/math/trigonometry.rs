use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};

use super::helpers::{num, require_number, require_two_numbers, unary_fn};

unary_fn!(Sin, |x: f64| x.sin());
unary_fn!(Cos, |x: f64| x.cos());
unary_fn!(Tan, |x: f64| x.tan());
unary_fn!(Asin, |x: f64| x.asin());
unary_fn!(Acos, |x: f64| x.acos());
unary_fn!(Atan, |x: f64| x.atan());
unary_fn!(Sinh, |x: f64| x.sinh());
unary_fn!(Cosh, |x: f64| x.cosh());
unary_fn!(Tanh, |x: f64| x.tanh());
unary_fn!(Asinh, |x: f64| x.asinh());
unary_fn!(Acosh, |x: f64| x.acosh());
unary_fn!(Atanh, |x: f64| x.atanh());
unary_fn!(Degrees, |x: f64| x.to_degrees());
unary_fn!(Radians, |x: f64| x.to_radians());

#[derive(Clone, Copy)]
pub struct Atan2;
impl Function<NonFunctionValue> for Atan2 {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (y, x) = require_two_numbers(args)?;
        Ok(num(y.atan2(x)))
    }
}

#[derive(Clone, Copy)]
pub struct Hypot;
impl Function<NonFunctionValue> for Hypot {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.hypot(b)))
    }
}
