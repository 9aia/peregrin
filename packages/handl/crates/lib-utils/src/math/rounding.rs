use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};

use super::helpers::{num, require_number, unary_fn};

unary_fn!(Abs, |x: f64| x.abs());
unary_fn!(Floor, |x: f64| x.floor());
unary_fn!(Ceil, |x: f64| x.ceil());
unary_fn!(Round, |x: f64| x.round());
unary_fn!(Sign, |x: f64| x.signum());
unary_fn!(Trunc, |x: f64| x.trunc());
unary_fn!(Fract, |x: f64| x.fract());

#[derive(Clone, Copy)]
pub struct RoundTo;
impl Function<NonFunctionValue> for RoundTo {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let value = require_number(&args[0])?;
        let decimals = require_number(&args[1])?;
        if decimals < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let multiplier = 10f64.powi(decimals as i32);
        Ok(num((value * multiplier).round() / multiplier))
    }
}
