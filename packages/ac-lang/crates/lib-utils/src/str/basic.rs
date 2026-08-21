use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{
    b, num, require_one_string, require_two_strings, s, string_predicate_fn, string_unary_fn,
};

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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(num(input.chars().count() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Concat;
impl Function for Concat {
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
        let mut result = String::new();
        for arg in args {
            let s = super::common::require_string(arg)?;
            result.push_str(s);
        }
        Ok(s(result))
    }
}

string_predicate_fn!(Equals, |input: &str, other: &str| input == other);

#[derive(Clone, Copy)]
pub struct IsEmpty;
impl Function for IsEmpty {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(b(input.is_empty()))
    }
}

#[derive(Clone, Copy)]
pub struct TrimMatches;
impl Function for TrimMatches {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, chars) = require_two_strings(args)?;
        if chars.is_empty() {
            return Ok(s(input.to_string()));
        }
        let char_set: Vec<char> = chars.chars().collect();
        Ok(s(input.trim_matches(|c| char_set.contains(&c)).to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct Bytes;
impl Function for Bytes {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(num(input.len() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct IsAlpha;
impl Function for IsAlpha {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(b(!input.is_empty() && input.chars().all(|c| c.is_alphabetic())))
    }
}

#[derive(Clone, Copy)]
pub struct IsNumeric;
impl Function for IsNumeric {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(b(!input.is_empty() && input.chars().all(|c| c.is_numeric())))
    }
}

#[derive(Clone, Copy)]
pub struct IsAlphanumeric;
impl Function for IsAlphanumeric {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        Ok(b(
            !input.is_empty() && input.chars().all(|c| c.is_alphanumeric()),
        ))
    }
}
