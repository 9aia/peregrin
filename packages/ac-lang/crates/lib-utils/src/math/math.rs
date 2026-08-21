use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

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
            _ => None,
        }
    }
}

fn require_number(arg: &RuntimeValue) -> Result<f64, Error> {
    arg.as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })
}

fn require_two_numbers(args: &[RuntimeValue]) -> Result<(f64, f64), Error> {
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

fn num(value: f64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(value))
}

macro_rules! unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Min;
impl Function for Min {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.min(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Max;
impl Function for Max {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.max(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Pow;
impl Function for Pow {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (base, exp) = require_two_numbers(args)?;
        Ok(num(base.powf(exp)))
    }
}

#[derive(Clone, Copy)]
pub struct Clamp;
impl Function for Clamp {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (y, x) = require_two_numbers(args)?;
        Ok(num(y.atan2(x)))
    }
}

#[derive(Clone, Copy)]
pub struct Hypot;
impl Function for Hypot {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.hypot(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Mod;
impl Function for Mod {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (a, b) = require_two_numbers(args)?;
        Ok(num(a.rem_euclid(b)))
    }
}

#[derive(Clone, Copy)]
pub struct Pi;
impl Function for Pi {
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        Ok(num(std::f64::consts::PI))
    }
}

#[derive(Clone, Copy)]
pub struct E;
impl Function for E {
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        Ok(num(std::f64::consts::E))
    }
}
