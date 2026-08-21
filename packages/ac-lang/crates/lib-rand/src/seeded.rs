use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde_json::Value;

use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{array, bool_val, num, require_array, require_int, require_number, s};
use crate::error::RandError;

fn seeded_rng(seed: f64) -> Result<StdRng, Error> {
    if !seed.is_finite() {
        return Err(RandError::InvalidRange {
            min: seed,
            max: seed,
        }
        .into());
    }
    let seed_u64 = if seed < 0.0 {
        (seed as i64).rem_euclid(1u64 as i64) as u64
    } else {
        seed as u64
    };
    Ok(StdRng::seed_from_u64(seed_u64))
}

/// `rand.seeded_int(seed: number, min: number, max: number) → number`
///
/// Returns a deterministic random integer in `[min, max]` based on `seed`.
/// Same seed + same args always produces the same result.
///
/// Examples:
/// - `rand.seeded_int(42, 1, 6)` → always the same die roll
/// - `rand.seeded_int(123, 0, 100)` → always the same value
#[derive(Clone, Copy)]
pub struct SeededInt;

impl Function for SeededInt {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let min = require_int(args, 1)?;
        let max = require_int(args, 2)?;
        if min > max {
            return Err(RandError::InvalidRange {
                min: min as f64,
                max: max as f64,
            }
            .into());
        }
        let mut rng = seeded_rng(seed)?;
        let result = rng.random_range(min..=max);
        Ok(num(result as f64))
    }
}

/// `rand.seeded_float(seed: number, min: number, max: number) → number`
///
/// Returns a deterministic random float in `[min, max)` based on `seed`.
///
/// Example:
/// - `rand.seeded_float(42, 0, 1)` → always the same value in [0, 1)
#[derive(Clone, Copy)]
pub struct SeededFloat;

impl Function for SeededFloat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let min = require_number(args, 1)?;
        let max = require_number(args, 2)?;
        if !min.is_finite() || !max.is_finite() {
            return Err(RandError::InvalidRange { min, max }.into());
        }
        if min > max {
            return Err(RandError::InvalidRange { min, max }.into());
        }
        if min == max {
            return Ok(num(min));
        }
        let mut rng = seeded_rng(seed)?;
        let result = rng.random_range(min..max);
        Ok(num(result))
    }
}

/// `rand.seeded_bool(seed: number) → bool`
///
/// Returns a deterministic random boolean based on `seed`.
///
/// Example:
/// - `rand.seeded_bool(42)` → always the same result
#[derive(Clone, Copy)]
pub struct SeededBool;

impl Function for SeededBool {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let mut rng = seeded_rng(seed)?;
        Ok(bool_val(rng.random()))
    }
}

/// `rand.seeded_pick(seed: number, array: array) → value`
///
/// Returns a single deterministic element from the array based on `seed`.
///
/// Example:
/// - `rand.seeded_pick(42, ["a", "b", "c"])` → always the same element
#[derive(Clone, Copy)]
pub struct SeededPick;

impl Function for SeededPick {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let arr = require_array(args, 1)?;
        if arr.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        let mut rng = seeded_rng(seed)?;
        let choice = arr.choose(&mut rng).expect("non-empty array");
        Ok(NonFunctionValue::new(choice.clone()))
    }
}

/// `rand.seeded_choice(seed: number, array: array, count: number) → array`
///
/// Returns `count` elements chosen deterministically **with replacement**
/// from the array based on `seed`.
///
/// Example:
/// - `rand.seeded_choice(42, [1, 2, 3], 5)` → always the same 5 elements
#[derive(Clone, Copy)]
pub struct SeededChoice;

impl Function for SeededChoice {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let arr = require_array(args, 1)?;
        let count = require_int(args, 2)?;

        if arr.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        if count < 0 {
            return Err(RandError::InvalidSampleCount {
                requested: count as usize,
                available: arr.len(),
            }
            .into());
        }

        let mut rng = seeded_rng(seed)?;
        let result: Vec<Value> = (0..count)
            .map(|_| arr.choose(&mut rng).expect("non-empty array").clone())
            .collect();
        Ok(array(result))
    }
}

/// `rand.seeded_sample(seed: number, array: array, count: number) → array`
///
/// Returns `count` elements chosen deterministically **without replacement**
/// from the array based on `seed`.
///
/// Example:
/// - `rand.seeded_sample(42, [1, 2, 3, 4, 5], 3)` → always the same 3 elements
#[derive(Clone, Copy)]
pub struct SeededSample;

impl Function for SeededSample {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let arr = require_array(args, 1)?;
        let count = require_int(args, 2)?;

        if count < 0 {
            return Err(RandError::InvalidSampleCount {
                requested: count as usize,
                available: arr.len(),
            }
            .into());
        }
        let count = count as usize;
        if count > arr.len() {
            return Err(RandError::InvalidSampleCount {
                requested: count,
                available: arr.len(),
            }
            .into());
        }

        let mut rng = seeded_rng(seed)?;
        let mut indices: Vec<usize> = (0..arr.len()).collect();
        indices.shuffle(&mut rng);
        let picked: Vec<Value> = indices[..count]
            .iter()
            .map(|&i| arr[i].clone())
            .collect();
        Ok(array(picked))
    }
}

/// `rand.seeded_shuffle(seed: number, array: array) → array`
///
/// Returns a new array with elements deterministically shuffled based on `seed`.
///
/// Example:
/// - `rand.seeded_shuffle(42, [1, 2, 3, 4, 5])` → always the same order
#[derive(Clone, Copy)]
pub struct SeededShuffle;

impl Function for SeededShuffle {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let arr = require_array(args, 1)?;
        let mut rng = seeded_rng(seed)?;
        let mut shuffled: Vec<Value> = arr.to_vec();
        shuffled.shuffle(&mut rng);
        Ok(array(shuffled))
    }
}

/// `rand.seeded_string(seed: number, length: number, charset?: string) → string`
///
/// Returns a deterministic random string of the given length based on `seed`.
///
/// Example:
/// - `rand.seeded_string(42, 16)` → always the same 16-char string
#[derive(Clone, Copy)]
pub struct SeededString;

impl Function for SeededString {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let length = require_int(args, 1)?;
        if length < 0 {
            return Err(RandError::InvalidStringLength { length }.into());
        }

        let charset_name = match args.len() {
            2 => "alphanumeric",
            3 => {
                let val = args.get(2).ok_or(Error::ArgumentCount {
                    expected: 3,
                    got: 2,
                })?;
                val.as_str().ok_or(Error::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })?
            }
            n => {
                return Err(Error::ArgumentCount {
                    expected: 3,
                    got: n,
                })
            }
        };

        let charset: &[u8] = match charset_name {
            "alphanumeric" => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            "alpha" => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            "numeric" => b"0123456789",
            "hex" => b"0123456789abcdef",
            "lower" => b"abcdefghijklmnopqrstuvwxyz",
            "upper" => b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            _ => {
                return Err(RandError::InvalidCharset {
                    name: charset_name.to_string(),
                }
                .into())
            }
        };

        let mut rng = seeded_rng(seed)?;
        let result: String = (0..length)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();
        Ok(s(result))
    }
}

/// `rand.seeded_gauss(seed: number, mean: number, stddev: number) → number`
///
/// Returns a deterministic random float from a Gaussian distribution
/// based on `seed`.
///
/// Example:
/// - `rand.seeded_gauss(42, 0, 1)` → always the same standard normal sample
#[derive(Clone, Copy)]
pub struct SeededGauss;

impl Function for SeededGauss {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let seed = require_number(args, 0)?;
        let mean = require_number(args, 1)?;
        let stddev = require_number(args, 2)?;
        if !stddev.is_finite() || !mean.is_finite() {
            return Err(RandError::InvalidRange {
                min: mean,
                max: stddev,
            }
            .into());
        }
        if stddev < 0.0 {
            return Err(RandError::InvalidRange {
                min: mean,
                max: stddev,
            }
            .into());
        }

        let mut rng = seeded_rng(seed)?;
        let u1: f64 = rng.random::<f64>().max(f64::MIN_POSITIVE);
        let u2: f64 = rng.random::<f64>();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        Ok(num(mean + stddev * z0))
    }
}
