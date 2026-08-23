use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{b, n, parse_iso, require_three_strings, require_two_strings};

#[derive(Clone, Copy)]
pub struct Compare;
impl Function for Compare {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a_str, b_str) = require_two_strings(args)?;
        let dt_a = parse_iso(a_str)?;
        let dt_b = parse_iso(b_str)?;
        let result = dt_a.cmp(&dt_b);
        let val = match result {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0};
        Ok(n(val))
    }
}

#[derive(Clone, Copy)]
pub struct IsBefore;
impl Function for IsBefore {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a_str, b_str) = require_two_strings(args)?;
        let dt_a = parse_iso(a_str)?;
        let dt_b = parse_iso(b_str)?;
        Ok(b(dt_a < dt_b))
    }
}

#[derive(Clone, Copy)]
pub struct IsAfter;
impl Function for IsAfter {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a_str, b_str) = require_two_strings(args)?;
        let dt_a = parse_iso(a_str)?;
        let dt_b = parse_iso(b_str)?;
        Ok(b(dt_a > dt_b))
    }
}

#[derive(Clone, Copy)]
pub struct IsEqual;
impl Function for IsEqual {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a_str, b_str) = require_two_strings(args)?;
        let dt_a = parse_iso(a_str)?;
        let dt_b = parse_iso(b_str)?;
        Ok(b(dt_a == dt_b))
    }
}

#[derive(Clone, Copy)]
pub struct IsBetween;
impl Function for IsBetween {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (target, start, end) = require_three_strings(args)?;
        let dt_target = parse_iso(target)?;
        let dt_start = parse_iso(start)?;
        let dt_end = parse_iso(end)?;
        Ok(b(dt_target > dt_start && dt_target < dt_end))
    }
}

#[derive(Clone, Copy)]
pub struct IsBetweenInclusive;
impl Function for IsBetweenInclusive {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (target, start, end) = require_three_strings(args)?;
        let dt_target = parse_iso(target)?;
        let dt_start = parse_iso(start)?;
        let dt_end = parse_iso(end)?;
        Ok(b(dt_target >= dt_start && dt_target <= dt_end))
    }
}
