use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{num, require_two_strings, string_predicate_fn};

string_predicate_fn!(Contains, |input: &str, pat: &str| input.contains(pat));
string_predicate_fn!(StartsWith, |input: &str, pat: &str| input.starts_with(pat));
string_predicate_fn!(EndsWith, |input: &str, pat: &str| input.ends_with(pat));

#[derive(Clone, Copy)]
pub struct Count;
impl Function for Count {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, pattern) = require_two_strings(args)?;
        Ok(num(input.matches(pattern).count() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Find;
impl Function for Find {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, pattern) = require_two_strings(args)?;
        let idx = input
            .rfind(pattern)
            .map(|byte_pos| input[..byte_pos].chars().count())
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        Ok(num(idx))
    }
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[derive(Clone, Copy)]
pub struct Levenshtein;
impl Function for Levenshtein {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let dist = levenshtein(&a_chars, &b_chars);
        Ok(num(dist as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Similarity;
impl Function for Similarity {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let dist = levenshtein(&a_chars, &b_chars);
        let max_len = a_chars.len().max(b_chars.len());
        if max_len == 0 {
            return Ok(num(1.0));
        }
        Ok(num(1.0 - (dist as f64 / max_len as f64)))
    }
}
