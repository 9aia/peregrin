use rand::prelude::IndexedRandom;
use rand::Rng;
use rand::seq::SliceRandom;
use serde_json::Value;

use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{array, bool_val, num, require_array, require_int, require_number, s};
use crate::error::RandError;

/// `rand.int(min: number, max: number) → number`
///
/// Returns a random integer in the inclusive range `[min, max]`.
///
/// Both `min` and `max` must be integers and `min` must be <= `max`.
///
/// Examples:
/// - `rand.int(1, 6)` → a die roll (1–6)
/// - `rand.int(0, 100)` → 0–100 inclusive
#[derive(Clone, Copy)]
pub struct Int;

impl Function for Int {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let min = require_int(args, 0)?;
        let max = require_int(args, 1)?;
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

/// `rand.float(min?: number, max?: number) → number`
///
/// Returns a random float. With no arguments, the result is in `[0, 1)`.
/// With two arguments, the result is in `[min, max)`.
///
/// Examples:
/// - `rand.float()` → 0.0 ≤ x < 1.0
/// - `rand.float(10, 20)` → 10.0 ≤ x < 20.0
#[derive(Clone, Copy)]
pub struct Float;

impl Function for Float {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        match args.len() {
            0 => {
                let result = rand::rng().random::<f64>();
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

impl Function for Bool {
    fn call(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Ok(bool_val(rand::rng().random()))
    }
}

/// `rand.pick(array: array) → value`
///
/// Returns a single random element from the array.
///
/// Returns an error if the array is empty.
///
/// Example:
/// - `rand.pick(["a", "b", "c"])` → one of `"a"`, `"b"`, `"c"`
#[derive(Clone, Copy)]
pub struct Pick;

impl Function for Pick {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        if arr.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        let choice = arr.choose(&mut rand::rng()).expect("non-empty array");
        Ok(NonFunctionValue::new(choice.clone()))
    }
}

/// `rand.sample(array: array, count: number) → array`
///
/// Returns `count` elements chosen uniformly at random **without
/// replacement** from the input array.
///
/// `count` must be a non-negative integer and must not exceed the array
/// length.
///
/// Example:
/// - `rand.sample([1, 2, 3, 4, 5], 3)` → 3 distinct elements in random order
#[derive(Clone, Copy)]
pub struct Sample;

impl Function for Sample {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        let count = require_int(args, 1)?;

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

        let mut indices: Vec<usize> = (0..arr.len()).collect();
        indices.shuffle(&mut rand::rng());
        let picked: Vec<Value> = indices[..count]
            .iter()
            .map(|&i| arr[i].clone())
            .collect();
        Ok(array(picked))
    }
}

/// `rand.shuffle(array: array) → array`
///
/// Returns a new array with the elements of the input in random order.
/// The original array is not modified (values are immutable in AC).
///
/// Example:
/// - `rand.shuffle([1, 2, 3, 4, 5])` → e.g. `[3, 1, 5, 2, 4]`
#[derive(Clone, Copy)]
pub struct Shuffle;

impl Function for Shuffle {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        let mut shuffled: Vec<Value> = arr.to_vec();
        shuffled.shuffle(&mut rand::rng());
        Ok(array(shuffled))
    }
}

/// `rand.weighted_pick(items: array, weights: array<number>) → value`
///
/// Returns a single element from `items` chosen with probability
/// proportional to the corresponding weight in `weights`.
///
/// Both arrays must have the same length, and at least one weight must be
/// positive.
///
/// Example:
/// - `rand.weighted_pick(["a", "b", "c"], [1, 1, 2])` → `"c"` twice as likely
#[derive(Clone, Copy)]
pub struct WeightedPick;

impl Function for WeightedPick {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let items = require_array(args, 0)?;
        let weights = require_array(args, 1)?;

        if items.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        if items.len() != weights.len() {
            return Err(RandError::WeightsLengthMismatch {
                items: items.len(),
                weights: weights.len(),
            }
            .into());
        }

        let weight_values: Vec<f64> = weights
            .iter()
            .map(|w| w.as_f64().unwrap_or(0.0))
            .collect();

        let total: f64 = weight_values.iter().sum();
        if total <= 0.0 || !total.is_finite() {
            return Err(RandError::NonPositiveWeights.into());
        }

        let mut rng = rand::rng();
        let threshold = rng.random::<f64>() * total;
        let mut cumulative = 0.0;
        for (i, &w) in weight_values.iter().enumerate() {
            cumulative += w;
            if threshold <= cumulative {
                return Ok(NonFunctionValue::new(items[i].clone()));
            }
        }

        // Fallback to last element (handles floating-point edge cases)
        Ok(NonFunctionValue::new(items[items.len() - 1].clone()))
    }
}

/// `rand.string(length: number) → string`
///
/// Returns a random string of the given length composed of alphanumeric
/// characters (`a-z`, `A-Z`, `0-9`).
///
/// `length` must be a non-negative integer.
///
/// Example:
/// - `rand.string(16)` → e.g. `"aB3xK9mN2pQr7sT1"`
#[derive(Clone, Copy)]
pub struct String_;

impl Function for String_ {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let length = require_int(args, 0)?;
        if length < 0 {
            return Err(RandError::InvalidStringLength { length }.into());
        }

        const CHARSET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let mut rng = rand::rng();
        let result: String = (0..length)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        Ok(s(result))
    }
}

/// `rand.bytes(length: number) → array<number>`
///
/// Returns an array of `length` random bytes (integers in the range 0–255).
///
/// `length` must be a non-negative integer.
///
/// Example:
/// - `rand.bytes(8)` → e.g. `[42, 255, 0, 128, 1, 64, 200, 17]`
#[derive(Clone, Copy)]
pub struct Bytes;

impl Function for Bytes {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let length = require_int(args, 0)?;
        if length < 0 {
            return Err(RandError::InvalidByteLength { length }.into());
        }

        let mut rng = rand::rng();
        let result: Vec<Value> = (0..length)
            .map(|_| Value::Number(serde_json::Number::from(rng.random::<u8>())))
            .collect();
        Ok(array(result))
    }
}

/// `rand.uuid() → string`
///
/// Returns a version-4 UUID string in the canonical 8-4-4-4-12 format.
///
/// Version and variant bits are set per RFC 4122. The randomness is
/// sourced from the system CSPRNG via the `rand` crate.
///
/// Example:
/// - `rand.uuid()` → `"f47ac10b-58cc-4372-a567-0e02b2c3d479"`
#[derive(Clone, Copy)]
pub struct Uuid;

impl Function for Uuid {
    fn call(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let mut rng = rand::rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);

        // Set version 4 (bits 12-15 of byte 6)
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        // Set variant 1 (bits 6-7 of byte 8)
        bytes[8] = (bytes[8] & 0x3F) | 0x80;

        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let uuid = format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32],
        );
        Ok(s(uuid))
    }
}
