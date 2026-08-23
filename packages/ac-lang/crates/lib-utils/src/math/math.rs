use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use ac_runtime_modules::Module;

/// The `math` module — numeric utility functions.
///
/// Provides functions callable as `math.abs(x)`, `math.floor(x)`, etc.
/// in AC expressions.
#[derive(Clone, Copy)]
pub struct MathModule;

impl Module for MathModule {
    fn name(&self) -> &str {
        "math"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "abs" => Some(&Abs),
            "floor" => Some(&Floor),
            "ceil" => Some(&Ceil),
            "round" => Some(&Round),
            "min" => Some(&Min),
            "max" => Some(&Max),
            "sqrt" => Some(&Sqrt),
            "pow" => Some(&Pow),
            "sign" => Some(&Sign),
            "trunc" => Some(&Trunc),
            "clamp" => Some(&Clamp),
            "log" => Some(&Log),
            "log10" => Some(&Log10),
            "exp" => Some(&Exp),
            "sin" => Some(&Sin),
            "cos" => Some(&Cos),
            "tan" => Some(&Tan),
            "asin" => Some(&Asin),
            "acos" => Some(&Acos),
            "atan" => Some(&Atan),
            "atan2" => Some(&Atan2),
            "hypot" => Some(&Hypot),
            "mod" => Some(&Mod),
            "pi" => Some(&Pi),
            "e" => Some(&E),
            "log2" => Some(&Log2),
            "cbrt" => Some(&Cbrt),
            "degrees" => Some(&Degrees),
            "radians" => Some(&Radians),
            "round_to" => Some(&RoundTo),
            "is_nan" => Some(&IsNan),
            "is_finite" => Some(&IsFinite),
            "is_infinite" => Some(&IsInfinite),
            "is_integer" => Some(&IsInteger),
            "nan" => Some(&Nan),
            "infinity" => Some(&Infinity),
            "tau" => Some(&Tau),
            "lerp" => Some(&Lerp),
            "map_range" => Some(&MapRange),
            "fract" => Some(&Fract),
            "gcd" => Some(&Gcd),
            "lcm" => Some(&Lcm),
            "sinh" => Some(&Sinh),
            "cosh" => Some(&Cosh),
            "tanh" => Some(&Tanh),
            "asinh" => Some(&Asinh),
            "acosh" => Some(&Acosh),
            "atanh" => Some(&Atanh),
            "factorial" => Some(&Factorial),
            "floor_div" => Some(&FloorDiv),
            _ => None,
        }
    }
}

fn require_number(arg: &NonFunctionValue) -> Result<f64, Error> {
    arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })
}

fn require_two_numbers(args: &[NonFunctionValue]) -> Result<(f64, f64), Error> {
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

fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

macro_rules! unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context,
            ) -> Result<NonFunctionValue, Error> {
                let arg = args.first().ok_or(Error::ArgumentCount {
                    expected: 1,
                    got: 0,
                })?;
                let x = require_number(arg)?;
                Ok(num($body(x)))
            }
        }
    };
}

macro_rules! bool_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context,
            ) -> Result<NonFunctionValue, Error> {
                let arg = args.first().ok_or(Error::ArgumentCount {
                    expected: 1,
                    got: 0,
                })?;
                let x = require_number(arg)?;
                Ok(b($body(x)))
            }
        }
    };
}

unary_fn!(Abs, |x: f64| x.abs());
unary_fn!(Floor, |x: f64| x.floor());
unary_fn!(Ceil, |x: f64| x.ceil());
unary_fn!(Round, |x: f64| x.round());
unary_fn!(Sqrt, |x: f64| x.sqrt());
unary_fn!(Sign, |x: f64| x.signum());
unary_fn!(Trunc, |x: f64| x.trunc());
unary_fn!(Log, |x: f64| x.ln());
unary_fn!(Log10, |x: f64| x.log10());
unary_fn!(Exp, |x: f64| x.exp());
unary_fn!(Sin, |x: f64| x.sin());
unary_fn!(Cos, |x: f64| x.cos());
unary_fn!(Tan, |x: f64| x.tan());
unary_fn!(Asin, |x: f64| x.asin());
unary_fn!(Acos, |x: f64| x.acos());
unary_fn!(Atan, |x: f64| x.atan());
unary_fn!(Log2, |x: f64| x.log2());
unary_fn!(Cbrt, |x: f64| x.cbrt());
unary_fn!(Degrees, |x: f64| x.to_degrees());
unary_fn!(Radians, |x: f64| x.to_radians());
unary_fn!(Fract, |x: f64| x.fract());
unary_fn!(Sinh, |x: f64| x.sinh());
unary_fn!(Cosh, |x: f64| x.cosh());
unary_fn!(Tanh, |x: f64| x.tanh());
unary_fn!(Asinh, |x: f64| x.asinh());
unary_fn!(Acosh, |x: f64| x.acosh());
unary_fn!(Atanh, |x: f64| x.atanh());

bool_unary_fn!(IsNan, |x: f64| x.is_nan());
bool_unary_fn!(IsFinite, |x: f64| x.is_finite());
bool_unary_fn!(IsInfinite, |x: f64| x.is_infinite());
bool_unary_fn!(IsInteger, |x: f64| x.is_finite() && x.fract() == 0.0);

#[derive(Clone, Copy)]
pub struct RoundTo;
impl Function for RoundTo {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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

#[derive(Clone, Copy)]
pub struct Min;
impl Function for Min {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.min(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Max;
impl Function for Max {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.max(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Pow;
impl Function for Pow {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (base, exp) = require_two_numbers(args)?;
        Ok(num(base.powf(exp)))
    }
}

#[derive(Clone, Copy)]
pub struct Clamp;
impl Function for Clamp {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
pub struct Atan2;
impl Function for Atan2 {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (y, x) = require_two_numbers(args)?;
        Ok(num(y.atan2(x)))
    }
}

#[derive(Clone, Copy)]
pub struct Hypot;
impl Function for Hypot {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.hypot(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Mod;
impl Function for Mod {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.rem_euclid(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Pi;
impl Function for Pi {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(std::f64::consts::PI))
    }
}

#[derive(Clone, Copy)]
pub struct E;
impl Function for E {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(std::f64::consts::E))
    }
}

#[derive(Clone, Copy)]
pub struct Nan;
impl Function for Nan {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(f64::NAN))
    }
}

#[derive(Clone, Copy)]
pub struct Infinity;
impl Function for Infinity {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(f64::INFINITY))
    }
}

#[derive(Clone, Copy)]
pub struct Tau;
impl Function for Tau {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(num(std::f64::consts::TAU))
    }
}

#[derive(Clone, Copy)]
pub struct Lerp;
impl Function for Lerp {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
impl Function for MapRange {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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

fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd_u64(b, a % b) }
}

#[derive(Clone, Copy)]
pub struct Gcd;
impl Function for Gcd {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
impl Function for Lcm {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
impl Function for Factorial {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
pub struct FloorDiv;
impl Function for FloorDiv {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
