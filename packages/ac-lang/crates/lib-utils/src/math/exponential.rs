use ac_runtime::{Context, Function};
use ac_runtime_values::{Error, NonFunctionValue};

use super::helpers::{num, require_number, require_two_numbers, unary_fn};

unary_fn!(Sqrt, |x: f64| x.sqrt());
unary_fn!(Log, |x: f64| x.ln());
unary_fn!(Log10, |x: f64| x.log10());
unary_fn!(Exp, |x: f64| x.exp());
unary_fn!(Log2, |x: f64| x.log2());
unary_fn!(Cbrt, |x: f64| x.cbrt());

#[derive(Clone, Copy)]
pub struct Pow;
impl Function<NonFunctionValue> for Pow {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (base, exp) = require_two_numbers(args)?;
        Ok(num(base.powf(exp)))
    }
}
