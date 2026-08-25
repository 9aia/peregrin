use handl_vm::Function;
use handl_vm_common::Module;
use handl_vm_values_json::NonFunctionValue;

use super::arithmetic::{
    Add, Clamp, Div, Factorial, FloorDiv, Gcd, Lcm, Lerp, MapRange, Max, Min, Mod, Mul, Sub,
};
use super::constants::{E, Infinity, Nan, Pi, Tau};
use super::exponential::{Cbrt, Exp, Log, Log2, Log10, Pow, Sqrt};
use super::predicates::{IsFinite, IsInfinite, IsInteger, IsNan};
use super::rounding::{Abs, Ceil, Floor, Fract, Round, RoundTo, Sign, Trunc};
use super::trigonometry::{
    Acos, Acosh, Asin, Asinh, Atan, Atan2, Atanh, Cos, Cosh, Degrees, Hypot, Radians, Sin, Sinh,
    Tan, Tanh,
};

/// The `math` module — numeric utility functions.
///
/// Provides functions callable as `math.abs(x)`, `math.floor(x)`, etc.
/// in handl expressions.
#[derive(Clone, Copy)]
pub struct MathModule;

impl Module for MathModule {
    fn name(&self) -> &str {
        "math"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "abs" => Some(&Abs),
            "add" => Some(&Add),
            "ceil" => Some(&Ceil),
            "clamp" => Some(&Clamp),
            "cos" => Some(&Cos),
            "cosh" => Some(&Cosh),
            "cbrt" => Some(&Cbrt),
            "asin" => Some(&Asin),
            "acos" => Some(&Acos),
            "atan" => Some(&Atan),
            "atan2" => Some(&Atan2),
            "asinh" => Some(&Asinh),
            "acosh" => Some(&Acosh),
            "atanh" => Some(&Atanh),
            "degrees" => Some(&Degrees),
            "div" => Some(&Div),
            "e" => Some(&E),
            "exp" => Some(&Exp),
            "factorial" => Some(&Factorial),
            "floor" => Some(&Floor),
            "floor_div" => Some(&FloorDiv),
            "fract" => Some(&Fract),
            "gcd" => Some(&Gcd),
            "hypot" => Some(&Hypot),
            "infinity" => Some(&Infinity),
            "is_finite" => Some(&IsFinite),
            "is_infinite" => Some(&IsInfinite),
            "is_integer" => Some(&IsInteger),
            "is_nan" => Some(&IsNan),
            "lcm" => Some(&Lcm),
            "lerp" => Some(&Lerp),
            "log" => Some(&Log),
            "log10" => Some(&Log10),
            "log2" => Some(&Log2),
            "map_range" => Some(&MapRange),
            "max" => Some(&Max),
            "min" => Some(&Min),
            "mod" => Some(&Mod),
            "mul" => Some(&Mul),
            "nan" => Some(&Nan),
            "pi" => Some(&Pi),
            "pow" => Some(&Pow),
            "radians" => Some(&Radians),
            "round" => Some(&Round),
            "round_to" => Some(&RoundTo),
            "sign" => Some(&Sign),
            "sin" => Some(&Sin),
            "sinh" => Some(&Sinh),
            "sqrt" => Some(&Sqrt),
            "sub" => Some(&Sub),
            "tan" => Some(&Tan),
            "tanh" => Some(&Tanh),
            "tau" => Some(&Tau),
            "trunc" => Some(&Trunc),
            _ => None,
        }
    }
}
