use ac_runtime::{Context, Function};
use ac_runtime_values::{Error, NonFunctionValue};

use super::helpers::{num, require_number, require_two_numbers};

#[derive(Clone, Copy)]
pub struct Add;
impl Function<NonFunctionValue> for Add {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a + b))
    }
}

#[derive(Clone, Copy)]
pub struct Sub;
impl Function<NonFunctionValue> for Sub {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a - b))
    }
}

#[derive(Clone, Copy)]
pub struct Mul;
impl Function<NonFunctionValue> for Mul {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a * b))
    }
}

#[derive(Clone, Copy)]
pub struct Div;
impl Function<NonFunctionValue> for Div {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        if b == 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-zero divisor",
                got: "zero",
            });
        }
        Ok(num(a / b))
    }
}

#[derive(Clone, Copy)]
pub struct Min;
impl Function<NonFunctionValue> for Min {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.min(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Max;
impl Function<NonFunctionValue> for Max {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.max(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Clamp;
impl Function<NonFunctionValue> for Clamp {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let value = require_number(&args[0])?;
        let min = require_number(&args[1])?;
        let max = require_number(&args[2])?;
        Ok(num(value.clamp(min, max)))
    }
}

#[derive(Clone, Copy)]
pub struct Mod;
impl Function<NonFunctionValue> for Mod {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.rem_euclid(b)))
    }
}

#[derive(Clone, Copy)]
pub struct FloorDiv;
impl Function<NonFunctionValue> for FloorDiv {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        if b == 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-zero divisor",
                got: "zero",
            });
        }
        Ok(num((a / b).floor()))
    }
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd_u64(b, a % b) }
}

#[derive(Clone, Copy)]
pub struct Gcd;
impl Function<NonFunctionValue> for Gcd {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        if a < 0.0 || b < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        Ok(num(gcd_u64(a as u64, b as u64) as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Lcm;
impl Function<NonFunctionValue> for Lcm {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        if a < 0.0 || b < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let a_u = a as u64;
        let b_u = b as u64;
        if a_u == 0 || b_u == 0 {
            return Ok(num(0.0));
        }
        Ok(num((a_u / gcd_u64(a_u, b_u) * b_u) as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Factorial;
impl Function<NonFunctionValue> for Factorial {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let x = require_number(arg)?;
        if x < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        if x.fract() != 0.0 {
            return Err(Error::TypeMismatch {
                expected: "integer",
                got: "non-integer",
            });
        }
        let n = x as u64;
        let mut result: f64 = 1.0;
        for i in 2..=n {
            result *= i as f64;
        }
        Ok(num(result))
    }
}

#[derive(Clone, Copy)]
pub struct Lerp;
impl Function<NonFunctionValue> for Lerp {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let start = require_number(&args[0])?;
        let end = require_number(&args[1])?;
        let t = require_number(&args[2])?;
        Ok(num(start + (end - start) * t))
    }
}

#[derive(Clone, Copy)]
pub struct MapRange;
impl Function<NonFunctionValue> for MapRange {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 5 {
            return Err(Error::ArgumentCount {
                expected: 5,
                got: args.len(),
            });
        }
        let value = require_number(&args[0])?;
        let in_min = require_number(&args[1])?;
        let in_max = require_number(&args[2])?;
        let out_min = require_number(&args[3])?;
        let out_max = require_number(&args[4])?;
        Ok(num(out_min
            + (value - in_min) * (out_max - out_min)
                / (in_max - in_min)))
    }
}
