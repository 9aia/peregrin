use ac_runtime::NonFunctionValue;
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
            "length" => Some(&Length),
            "concat" => Some(&Concat),
            "replace_first" => Some(&ReplaceFirst),
            _ => None,
        }
    }
}

fn require_string(arg: &NonFunctionValue) -> Result<&str, Error> {
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

fn require_one_string(args: &[NonFunctionValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_string(arg)
}

fn require_two_strings(args: &[NonFunctionValue]) -> Result<(&str, &str), Error> {
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

fn s(value: impl Into<String>) -> NonFunctionValue {
    NonFunctionValue::new(value.into())
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

macro_rules! string_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut Context,
            ) -> Result<NonFunctionValue, Error> {
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
pub struct Length;
impl Function for Length {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(num(input.chars().count() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Split;
impl Function for Split {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, sep) = require_two_strings(args)?;
        let parts: Vec<serde_json::Value> = input
            .split(sep)
            .map(|part| serde_json::Value::String(part.to_string()))
            .collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(parts)))
    }
}

#[derive(Clone, Copy)]
pub struct Join;
impl Function for Join {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
                args: &[NonFunctionValue],
                _ctx: &mut Context,
            ) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, pattern) = require_two_strings(args)?;
        let idx = input
            .find(pattern)
            .map(|byte_pos| input[..byte_pos].chars().count())
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        Ok(num(idx))
    }
}

#[derive(Clone, Copy)]
pub struct Repeat;
impl Function for Repeat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
        })?;
        if count < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        Ok(s(input.repeat(count as usize)))
    }
}

#[derive(Clone, Copy)]
pub struct CharAt;
impl Function for CharAt {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
        })?;
        if index < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        match input.chars().nth(index as usize) {
            Some(ch) => Ok(s(ch.to_string())),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Slice;
impl Function for Slice {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
        })?;
        let end = args[2].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if start < 0.0 || end < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let chars: Vec<char> = input.chars().collect();
        let end = (end as usize).min(chars.len());
        let start = (start as usize).min(end);
        Ok(s(chars[start..end].iter().collect::<String>()))
    }
}

fn pad_to_len(input: &str, target_len: usize, pad_char: &str, pad_left: bool) -> String {
    let char_count = input.chars().count();
    if char_count >= target_len || pad_char.is_empty() {
        return input.to_string();
    }
    let pad_total = target_len - char_count;
    let pad_chars: Vec<char> = pad_char.chars().collect();
    let pad_repeated: String = pad_chars
        .iter()
        .cycle()
        .take(pad_total)
        .collect();
    if pad_left {
        format!("{pad_repeated}{input}")
    } else {
        format!("{input}{pad_repeated}")
    }
}

#[derive(Clone, Copy)]
pub struct Concat;
impl Function for Concat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let mut result = String::new();
        for arg in args {
            let s = require_string(arg)?;
            result.push_str(s);
        }
        Ok(s(result))
    }
}

#[derive(Clone, Copy)]
pub struct ReplaceFirst;
impl Function for ReplaceFirst {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let input = require_string(&args[0])?;
        let from = require_string(&args[1])?;
        let to = require_string(&args[2])?;
        Ok(s(input.replacen(from, to, 1)))
    }
}

#[derive(Clone, Copy)]
pub struct PadLeft;
impl Function for PadLeft {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
        })?;
        if target_len < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len as usize, pad_char, true)))
    }
}

#[derive(Clone, Copy)]
pub struct PadRight;
impl Function for PadRight {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
        })?;
        if target_len < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len as usize, pad_char, false)))
    }
}
