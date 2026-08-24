use ac_runtime_values::NonFunctionValue;

use ac_runtime::{Context, Function};
use ac_runtime_values::{Error};


use super::helpers::array_unary_fn;
use super::helpers::{arr, b, num, require_array, require_one_array, require_two_arrays};

array_unary_fn!(
    Length,
    |input: &[serde_json::Value]| num(input.len() as f64)
);
array_unary_fn!(Reverse, |input: &[serde_json::Value]| {
    let reversed: Vec<serde_json::Value> = input.iter().rev().cloned().collect();
    arr(&reversed)
});
array_unary_fn!(First, |input: &[serde_json::Value]| {
    match input.first() {
        Some(v) => NonFunctionValue::new(v.clone()),
        None => NonFunctionValue::new(serde_json::Value::Null),
    }
});
array_unary_fn!(Last, |input: &[serde_json::Value]| {
    match input.last() {
        Some(v) => NonFunctionValue::new(v.clone()),
        None => NonFunctionValue::new(serde_json::Value::Null),
    }
});
array_unary_fn!(Flatten, |input: &[serde_json::Value]| {
    let mut flat = Vec::new();
    for item in input {
        match item {
            serde_json::Value::Array(nested) => flat.extend(nested.iter().cloned()),
            other => flat.push(other.clone()),
        }
    }
    arr(&flat)
});

#[derive(Clone, Copy)]
pub struct DeepFlatten;
impl Function<NonFunctionValue> for DeepFlatten {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        fn deep_flat(items: &[serde_json::Value], out: &mut Vec<serde_json::Value>) {
            for item in items {
                match item {
                    serde_json::Value::Array(nested) => deep_flat(nested, out),
                    other => out.push(other.clone()),
                }
            }
        }
        let mut flat = Vec::new();
        deep_flat(input, &mut flat);
        Ok(arr(&flat))
    }
}

#[derive(Clone, Copy)]
pub struct Concat;
impl Function<NonFunctionValue> for Concat {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let combined: Vec<serde_json::Value> = a.iter().chain(b.iter()).cloned().collect();
        Ok(arr(&combined))
    }
}

#[derive(Clone, Copy)]
pub struct Contains;
impl Function<NonFunctionValue> for Contains {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let needle = &args[1];
        let needle_inner = needle.inner();
        let found = array.iter().any(|item| item == needle_inner);
        Ok(b(found))
    }
}

#[derive(Clone, Copy)]
pub struct IndexOf;
impl Function<NonFunctionValue> for IndexOf {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let needle = &args[1];
        let needle_inner = needle.inner();
        let idx = array
            .iter()
            .position(|item| item == needle_inner)
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        Ok(num(idx))
    }
}

#[derive(Clone, Copy)]
pub struct Slice;
impl Function<NonFunctionValue> for Slice {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
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
        let start = start as usize;
        let end = end as usize;
        let end = end.min(array.len());
        let start = start.min(end);
        Ok(arr(&array[start..end]))
    }
}

#[derive(Clone, Copy)]
pub struct Sort;
impl Function<NonFunctionValue> for Sort {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        let mut sorted: Vec<serde_json::Value> = input.to_vec();
        sorted.sort_by(|a, c| match (a.as_f64(), c.as_f64()) {
            (Some(an), Some(cn)) => an.partial_cmp(&cn).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => match (a.as_str(), c.as_str()) {
                (Some(as_), Some(cs)) => as_.cmp(cs),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            },
        });
        Ok(arr(&sorted))
    }
}

#[derive(Clone, Copy)]
pub struct At;
impl Function<NonFunctionValue> for At {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
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
        match array.get(index as usize) {
            Some(v) => Ok(NonFunctionValue::new(v.clone())),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Push;
impl Function<NonFunctionValue> for Push {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let mut result = array.to_vec();
        for arg in &args[1..] {
            result.push(arg.inner().clone());
        }
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct Fill;
impl Function<NonFunctionValue> for Fill {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let count = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if count < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let value = args[1].inner().clone();
        let result: Vec<serde_json::Value> = (0..count as usize).map(|_| value.clone()).collect();
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct Pop;
impl Function<NonFunctionValue> for Pop {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let array = require_one_array(args)?;
        if array.is_empty() {
            return Ok(arr(&[]));
        }
        Ok(arr(&array[..array.len() - 1]))
    }
}

#[derive(Clone, Copy)]
pub struct Shift;
impl Function<NonFunctionValue> for Shift {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let array = require_one_array(args)?;
        if array.is_empty() {
            return Ok(arr(&[]));
        }
        Ok(arr(&array[1..]))
    }
}
