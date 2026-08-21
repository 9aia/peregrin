use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

/// The `string` module — string utility functions.
///
/// Provides functions callable as `string.upper(s)`, `string.split(s, sep)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct StringModule;

impl Module for StringModule {
    fn name(&self) -> &str {
        "string"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "upper" => Some(&Upper),
            "lower" => Some(&Lower),
            "trim" => Some(&Trim),
            "split" => Some(&Split),
            "join" => Some(&Join),
            "replace" => Some(&Replace),
            "contains" => Some(&Contains),
            "starts_with" => Some(&StartsWith),
            "ends_with" => Some(&EndsWith),
            "slice" => Some(&Slice),
            "reverse" => Some(&Reverse),
            "repeat" => Some(&Repeat),
            "find" => Some(&Find),
            "trim_start" => Some(&TrimStart),
            "trim_end" => Some(&TrimEnd),
            "count" => Some(&Count),
            "char_at" => Some(&CharAt),
            "pad_left" => Some(&PadLeft),
            "pad_right" => Some(&PadRight),
            _ => None,
        }
    }
}

fn require_string(arg: &RuntimeValue) -> Result<&str, Error> {
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

fn require_one_string(args: &[RuntimeValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_string(arg)
}

fn require_two_strings(args: &[RuntimeValue]) -> Result<(&str, &str), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_string(&args[0])?;
    let b = require_string(&args[1])?;
    Ok((a, b))
}

fn s(value: impl Into<String>) -> RuntimeValue {
    RuntimeValue::new(value.into())
}

fn b(value: bool) -> RuntimeValue {
    RuntimeValue::new(value)
}

fn num(value: f64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(value))
}

macro_rules! string_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
                let input = require_one_string(args)?;
                Ok(s($body(input)))
            }
        }
    };
}

string_unary_fn!(Upper, |input: &str| input.to_uppercase());
string_unary_fn!(Lower, |input: &str| input.to_lowercase());
string_unary_fn!(Trim, |input: &str| input.trim().to_string());
string_unary_fn!(TrimStart, |input: &str| input.trim_start().to_string());
string_unary_fn!(TrimEnd, |input: &str| input.trim_end().to_string());
string_unary_fn!(Reverse, |input: &str| input
    .chars()
    .rev()
    .collect::<String>());

#[derive(Clone, Copy)]
pub struct Split;
impl Function for Split {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (input, sep) = require_two_strings(args)?;
        let parts: Vec<serde_json::Value> = input
            .split(sep)
            .map(|part| serde_json::Value::String(part.to_string()))
            .collect();
        Ok(RuntimeValue::new(serde_json::Value::Array(parts)))
    }
}

#[derive(Clone, Copy)]
pub struct Join;
impl Function for Join {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let sep = require_string(&args[1])?;
        let arr = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let parts: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
        Ok(s(parts.join(sep)))
    }
}

#[derive(Clone, Copy)]
pub struct Replace;
impl Function for Replace {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let from = require_string(&args[1])?;
        let to = require_string(&args[2])?;
        Ok(s(input.replace(from, to)))
    }
}

macro_rules! string_predicate_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
                let (input, pattern) = require_two_strings(args)?;
                Ok(b($body(input, pattern)))
            }
        }
    };
}

string_predicate_fn!(Contains, |input: &str, pat: &str| input.contains(pat));
string_predicate_fn!(StartsWith, |input: &str, pat: &str| input.starts_with(pat));
string_predicate_fn!(EndsWith, |input: &str, pat: &str| input.ends_with(pat));

#[derive(Clone, Copy)]
pub struct Count;
impl Function for Count {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (input, pattern) = require_two_strings(args)?;
        if pattern.is_empty() {
            return Ok(num(0.0));
        }
        Ok(num(input.matches(pattern).count() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Find;
impl Function for Find {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (input, pattern) = require_two_strings(args)?;
        let idx = input.find(pattern).map(|i| i as f64).unwrap_or(-1.0);
        Ok(num(idx))
    }
}

#[derive(Clone, Copy)]
pub struct Repeat;
impl Function for Repeat {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let count = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        Ok(s(input.repeat(count)))
    }
}

#[derive(Clone, Copy)]
pub struct CharAt;
impl Function for CharAt {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let index = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        match input.chars().nth(index) {
            Some(ch) => Ok(s(ch.to_string())),
            None => Ok(RuntimeValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Slice;
impl Function for Slice {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let start = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        let end = args[2].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        let end = end.min(input.len());
        let start = start.min(end);
        Ok(s(input[start..end].to_string()))
    }
}

fn pad_to_len(input: &str, target_len: usize, pad_char: &str, pad_left: bool) -> String {
    if input.len() >= target_len || pad_char.is_empty() {
        return input.to_string();
    }
    let pad_total = target_len - input.len();
    let pad_repeated = pad_char.repeat(pad_total / pad_char.len() + 1);
    let pad = &pad_repeated[..pad_total];
    if pad_left {
        format!("{pad}{input}")
    } else {
        format!("{input}{pad}")
    }
}

#[derive(Clone, Copy)]
pub struct PadLeft;
impl Function for PadLeft {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let target_len = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len, pad_char, true)))
    }
}

#[derive(Clone, Copy)]
pub struct PadRight;
impl Function for PadRight {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let target_len = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len, pad_char, false)))
    }
}
