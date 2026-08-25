//! `collections.hash_set` — unordered set of unique values.
//!
//! Implemented as a deduplicated array of `serde_json::Value`.

use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};

use super::common::{arr, b, num, require_array, require_one_array, require_two_arrays};

fn has_value(input: &[serde_json::Value], target: &serde_json::Value) -> bool {
    input.iter().any(|item| item == target)
}

/// `collections.hash_set.new() → set`
#[derive(Clone, Copy)]
pub struct New;

impl Function<NonFunctionValue> for New {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(arr(&[]))
    }
}

/// `collections.hash_set.from(array: array) → set`
#[derive(Clone, Copy)]
pub struct From;

impl Function<NonFunctionValue> for From {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        let mut values: Vec<serde_json::Value> = Vec::new();
        for item in input {
            if !has_value(&values, item) {
                values.push(item.clone());
            }
        }
        Ok(arr(&values))
    }
}

/// `collections.hash_set.to_array(set: set) → array`
#[derive(Clone, Copy)]
pub struct ToArray;

impl Function<NonFunctionValue> for ToArray {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        Ok(arr(input))
    }
}

/// `collections.hash_set.len(set: set) → number`
#[derive(Clone, Copy)]
pub struct Len;

impl Function<NonFunctionValue> for Len {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        Ok(num(input.len() as f64))
    }
}

/// `collections.hash_set.is_empty(set: set) → bool`
#[derive(Clone, Copy)]
pub struct IsEmpty;

impl Function<NonFunctionValue> for IsEmpty {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        Ok(b(input.is_empty()))
    }
}

/// `collections.hash_set.add(set: set, value: value) → set`
#[derive(Clone, Copy)]
pub struct Add;

impl Function<NonFunctionValue> for Add {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_array(args, 0)?;
        let value = args.get(1).ok_or(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        })?;
        let target = value.inner();
        let mut values = input.to_vec();
        if !has_value(&values, target) {
            values.push(target.clone());
        }
        Ok(arr(&values))
    }
}

/// `collections.hash_set.remove(set: set, value: value) → set`
#[derive(Clone, Copy)]
pub struct Remove;

impl Function<NonFunctionValue> for Remove {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_array(args, 0)?;
        let value = args.get(1).ok_or(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        })?;
        let target = value.inner();
        let mut values = input.to_vec();
        if let Some(pos) = values.iter().position(|item| item == target) {
            values.remove(pos);
        }
        Ok(arr(&values))
    }
}

/// `collections.hash_set.has(set: set, value: value) → bool`
#[derive(Clone, Copy)]
pub struct Has;

impl Function<NonFunctionValue> for Has {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_array(args, 0)?;
        let value = args.get(1).ok_or(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        })?;
        Ok(b(has_value(input, value.inner())))
    }
}

/// `collections.hash_set.union(a: set, b: set) → set`
#[derive(Clone, Copy)]
pub struct Union;

impl Function<NonFunctionValue> for Union {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let mut values: Vec<serde_json::Value> = Vec::new();
        for item in a.iter().chain(b.iter()) {
            if !has_value(&values, item) {
                values.push(item.clone());
            }
        }
        Ok(arr(&values))
    }
}

/// `collections.hash_set.intersection(a: set, b: set) → set`
#[derive(Clone, Copy)]
pub struct Intersection;

impl Function<NonFunctionValue> for Intersection {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let mut values: Vec<serde_json::Value> = Vec::new();
        for item in a {
            if has_value(b, item) && !has_value(&values, item) {
                values.push(item.clone());
            }
        }
        Ok(arr(&values))
    }
}

/// `collections.hash_set.difference(a: set, b: set) → set`
#[derive(Clone, Copy)]
pub struct Difference;

impl Function<NonFunctionValue> for Difference {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let mut values: Vec<serde_json::Value> = Vec::new();
        for item in a {
            if !has_value(b, item) && !has_value(&values, item) {
                values.push(item.clone());
            }
        }
        Ok(arr(&values))
    }
}
