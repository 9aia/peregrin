use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{num, require_two_strings, string_predicate_fn};

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
pub struct RFind;
impl Function for RFind {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, pattern) = require_two_strings(args)?;
        let idx = input
            .rfind(pattern)
            .map(|byte_pos| input[..byte_pos].chars().count())
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        Ok(num(idx))
    }
}
