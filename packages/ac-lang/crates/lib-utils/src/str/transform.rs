use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{require_one_string, require_string, require_two_strings, s};

#[derive(Clone, Copy)]
pub struct Split;
impl Function for Split {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct ReplaceFirst;
impl Function for ReplaceFirst {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
pub struct Repeat;
impl Function for Repeat {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
        if !count.is_finite() || count > 1_000_000.0 {
            return Err(Error::TypeMismatch {
                expected: "finite number <= 1_000_000",
                got: "non-finite or excessively large number",
            });
        }
        Ok(s(input.repeat(count as usize)))
    }
}

#[derive(Clone, Copy)]
pub struct CharAt;
impl Function for CharAt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
    let pad_repeated: String = pad_chars.iter().cycle().take(pad_total).collect();
    if pad_left {
        format!("{pad_repeated}{input}")
    } else {
        format!("{input}{pad_repeated}")
    }
}

#[derive(Clone, Copy)]
pub struct PadLeft;
impl Function for PadLeft {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
        if !target_len.is_finite() || target_len > 1_000_000.0 {
            return Err(Error::TypeMismatch {
                expected: "finite number <= 1_000_000",
                got: "non-finite or excessively large number",
            });
        }
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len as usize, pad_char, true)))
    }
}

#[derive(Clone, Copy)]
pub struct PadRight;
impl Function for PadRight {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
        if !target_len.is_finite() || target_len > 1_000_000.0 {
            return Err(Error::TypeMismatch {
                expected: "finite number <= 1_000_000",
                got: "non-finite or excessively large number",
            });
        }
        let pad_char = require_string(&args[2])?;
        Ok(s(pad_to_len(input, target_len as usize, pad_char, false)))
    }
}

#[derive(Clone, Copy)]
pub struct StripPrefix;
impl Function for StripPrefix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, prefix) = require_two_strings(args)?;
        Ok(s(input.strip_prefix(prefix).unwrap_or(input).to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct StripSuffix;
impl Function for StripSuffix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, suffix) = require_two_strings(args)?;
        Ok(s(input.strip_suffix(suffix).unwrap_or(input).to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct Lines;
impl Function for Lines {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let lines: Vec<serde_json::Value> = if input.is_empty() {
            vec![serde_json::Value::String(String::new())]
        } else {
            input
                .lines()
                .map(|line| serde_json::Value::String(line.to_string()))
                .collect()
        };
        Ok(NonFunctionValue::new(serde_json::Value::Array(lines)))
    }
}

#[derive(Clone, Copy)]
pub struct SplitAt;
impl Function for SplitAt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
        let chars: Vec<char> = input.chars().collect();
        let mid = (index as usize).min(chars.len());
        let left: String = chars[..mid].iter().collect();
        let right: String = chars[mid..].iter().collect();
        let parts = vec![
            serde_json::Value::String(left),
            serde_json::Value::String(right),
        ];
        Ok(NonFunctionValue::new(serde_json::Value::Array(parts)))
    }
}
