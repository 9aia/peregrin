use ac_runtime_values::NonFunctionValue;
use rand::Rng;

use ac_runtime_values::{Error};
use ac_runtime::{Context, Function};

use super::common::{bool_val, num, require_int, require_number};
use crate::error::RandError;

/// `rand.int(max: number) → number` / `rand.int(min: number, max: number) → number`
///
/// Returns a random integer in the inclusive range `[0, max]` (one-arg)
/// or `[min, max]` (two-arg).
///
/// Both `min` and `max` must be integers and `min` must be <= `max`.
///
/// Examples:
/// - `rand.int(6)` → 0–6 inclusive
/// - `rand.int(1, 6)` → a die roll (1–6)
/// - `rand.int(0, 100)` → 0–100 inclusive
#[derive(Clone, Copy)]
pub struct Int;

impl Function<NonFunctionValue> for Int {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (min, max) = match args.len() {
            1 => (0, require_int(args, 0)?),
            2 => (require_int(args, 0)?, require_int(args, 1)?),
            n => {
                return Err(Error::ArgumentCount {
                    expected: 2,
                    got: n,
                });
            }
        };
        if min > max {
            return Err(RandError::InvalidRange {
                min: min as f64,
                max: max as f64,
            }
            .into());
        }
        let result = rand::rng().random_range(min..=max);
        Ok(num(result as f64))
    }
}

/// `rand.float() → number` / `rand.float(max: number) → number` / `rand.float(min: number, max: number) → number`
///
/// Returns a random float. With no arguments, the result is in `[0, 1)`.
/// With one argument, the result is in `[0, max)`.
/// With two arguments, the result is in `[min, max)`.
///
/// Examples:
/// - `rand.float()` → 0.0 ≤ x < 1.0
/// - `rand.float(10)` → 0.0 ≤ x < 10.0
/// - `rand.float(10, 20)` → 10.0 ≤ x < 20.0
#[derive(Clone, Copy)]
pub struct Float;

impl Function<NonFunctionValue> for Float {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        match args.len() {
            0 => {
                let result = rand::rng().random::<f64>();
                Ok(num(result))
            }
            1 => {
                let max = require_number(args, 0)?;
                if !max.is_finite() {
                    return Err(RandError::InvalidRange { min: 0.0, max }.into());
                }
                if max <= 0.0 {
                    return Err(RandError::InvalidRange { min: 0.0, max }.into());
                }
                let result = rand::rng().random_range(0.0..max);
                Ok(num(result))
            }
            2 => {
                let min = require_number(args, 0)?;
                let max = require_number(args, 1)?;
                if !min.is_finite() || !max.is_finite() {
                    return Err(RandError::InvalidRange { min, max }.into());
                }
                if min > max {
                    return Err(RandError::InvalidRange { min, max }.into());
                }
                if min == max {
                    return Ok(num(min));
                }
                let result = rand::rng().random_range(min..max);
                Ok(num(result))
            }
            n => Err(Error::ArgumentCount {
                expected: 2,
                got: n,
            }),
        }
    }
}

/// `rand.bool() → bool`
///
/// Returns a random boolean with equal probability for `true` and `false`.
///
/// Example:
/// - `rand.bool()` → `true` or `false`
#[derive(Clone, Copy)]
pub struct Bool;

impl Function<NonFunctionValue> for Bool {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(bool_val(rand::rng().random()))
    }
}