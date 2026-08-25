use handl_vm_values_json::NonFunctionValue;
use rand::Rng;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use serde_json::Value;

use handl_vm::{Context, Function};
use handl_vm_values_json::Error;

use super::common::{array, require_array, require_int};
use crate::error::RandError;

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

impl Function<NonFunctionValue> for Pick {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        if arr.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        let choice = arr.choose(&mut rand::rng()).expect("non-empty array");
        Ok(NonFunctionValue::new(choice.clone()))
    }
}

/// `rand.choice(array: array, count: number) → array`
///
/// Returns `count` elements chosen uniformly at random **with
/// replacement** from the input array. Duplicate elements may appear.
///
/// `count` must be a non-negative integer. The array must be non-empty.
///
/// Example:
/// - `rand.choice([1, 2, 3], 5)` → 5 elements, possibly with duplicates
#[derive(Clone, Copy)]
pub struct Choice;

impl Function<NonFunctionValue> for Choice {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        let count = require_int(args, 1)?;

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

        let mut rng = rand::rng();
        let result: Vec<Value> = (0..count)
            .map(|_| arr.choose(&mut rng).expect("non-empty array").clone())
            .collect();
        Ok(array(result))
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

impl Function<NonFunctionValue> for Sample {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
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
        let picked: Vec<Value> = indices[..count].iter().map(|&i| arr[i].clone()).collect();
        Ok(array(picked))
    }
}

/// `rand.shuffle(array: array) → array`
///
/// Returns a new array with the elements of the input in random order.
/// The original array is not modified (values are immutable in handl).
///
/// Example:
/// - `rand.shuffle([1, 2, 3, 4, 5])` → e.g. `[3, 1, 5, 2, 4]`
#[derive(Clone, Copy)]
pub struct Shuffle;

impl Function<NonFunctionValue> for Shuffle {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
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

impl Function<NonFunctionValue> for WeightedPick {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
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
            .enumerate()
            .map(|(i, w)| w.as_f64().ok_or(RandError::NonNumericWeight { index: i }))
            .collect::<Result<_, _>>()?;

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

        Ok(NonFunctionValue::new(items[items.len() - 1].clone()))
    }
}
