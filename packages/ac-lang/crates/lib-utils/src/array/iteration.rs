use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::helpers::{arr, array_unary_fn, num, require_array, require_function_value};
use crate::shared::{apply_fn, apply_fn2, find_index, is_truthy_fn};

// ── take / drop ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Take;
impl Function for Take {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let n = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if n < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let n = (n as usize).min(array.len());
        Ok(arr(&array[..n]))
    }
}

#[derive(Clone, Copy)]
pub struct Drop;
impl Function for Drop {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let n = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if n < 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-negative number",
                got: "negative number",
            });
        }
        let n = (n as usize).min(array.len());
        Ok(arr(&array[n..]))
    }
}

// ── enumerate / unzip ──────────────────────────────────────────────

array_unary_fn!(Enumerate, |input: &[serde_json::Value]| {
    let pairs: Vec<serde_json::Value> = input
        .iter()
        .enumerate()
        .map(|(i, v)| serde_json::json!([i, v.clone()]))
        .collect();
    arr(&pairs)
});

array_unary_fn!(Unzip, |input: &[serde_json::Value]| {
    let mut firsts = Vec::new();
    let mut seconds = Vec::new();
    for item in input {
        if let Some(pair) = item.as_array() {
            if pair.len() >= 2 {
                firsts.push(pair[0].clone());
                seconds.push(pair[1].clone());
            }
        }
    }
    arr(&[
        serde_json::Value::Array(firsts),
        serde_json::Value::Array(seconds),
    ])
});

// ── intersperse / intercalate ──────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Intersperse;
impl Function for Intersperse {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let sep = args[1].inner().clone();
        let mut result = Vec::with_capacity(array.len().saturating_mul(2));
        for (i, item) in array.iter().enumerate() {
            if i > 0 {
                result.push(sep.clone());
            }
            result.push(item.clone());
        }
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct Intercalate;
impl Function for Intercalate {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let sep = require_array(&args[1])?;
        let mut result = Vec::new();
        for (i, item) in array.iter().enumerate() {
            if i > 0 {
                result.extend(sep.iter().cloned());
            }
            if let Some(nested) = item.as_array() {
                result.extend(nested.iter().cloned());
            } else {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── dedupe / prepend ─────────────────────────────────────────────

array_unary_fn!(Dedupe, |input: &[serde_json::Value]| {
    let mut result = Vec::new();
    for item in input {
        if result.last() != Some(item) {
            result.push(item.clone());
        }
    }
    arr(&result)
});

#[derive(Clone, Copy)]
pub struct Prepend;
impl Function for Prepend {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let mut result = Vec::with_capacity(array.len() + 1);
        result.push(args[1].inner().clone());
        result.extend(array.iter().cloned());
        Ok(arr(&result))
    }
}

// ── take_while / drop_while ────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct TakeWhile;
impl Function for TakeWhile {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let mut result = Vec::new();
        for item in array {
            if is_truthy_fn(ctx, &fn_val, item)? {
                result.push(item.clone());
            } else {
                break;
            }
        }
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct DropWhile;
impl Function for DropWhile {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let mut idx = 0;
        for item in array {
            if is_truthy_fn(ctx, &fn_val, item)? {
                idx += 1;
            } else {
                break;
            }
        }
        Ok(arr(&array[idx..]))
    }
}

// ── scan / foldr ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Scan;
impl Function for Scan {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let mut acc = args[2].inner().clone();
        let mut results = Vec::with_capacity(array.len());
        for item in array {
            let result_val = apply_fn2(ctx, &fn_val, &acc, item)?;
            acc = result_val.into_inner();
            results.push(acc.clone());
        }
        Ok(arr(&results))
    }
}

#[derive(Clone, Copy)]
pub struct Foldr;
impl Function for Foldr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let mut acc = args[2].clone();
        for item in array.iter().rev() {
            acc = apply_fn2(ctx, &fn_val, acc.inner(), item)?;
        }
        Ok(acc)
    }
}

// ── find_index / find_last / find_last_index ───────────────────────

#[derive(Clone, Copy)]
pub struct FindIndex;
impl Function for FindIndex {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let idx = find_index(ctx, &fn_val, array.iter())?;
        Ok(num(idx.map(|i| i as f64).unwrap_or(-1.0)))
    }
}

#[derive(Clone, Copy)]
pub struct FindLast;
impl Function for FindLast {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let result = crate::shared::find_first_rev(ctx, &fn_val, array.iter().rev())?;
        match result {
            Some(v) => Ok(NonFunctionValue::new(v)),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct FindLastIndex;
impl Function for FindLastIndex {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let idx = crate::shared::find_index_rev(ctx, &fn_val, array.iter().rev())?;
        Ok(num(idx
            .map(|i| (array.len() - 1 - i) as f64)
            .unwrap_or(-1.0)))
    }
}

// ── windows / transpose ────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Windows;
impl Function for Windows {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let size = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if size < 1.0 {
            return Err(Error::TypeMismatch {
                expected: "positive number",
                got: "non-positive number",
            });
        }
        let size = size as usize;
        if size > array.len() {
            return Ok(arr(&[]));
        }
        let windows: Vec<serde_json::Value> = array
            .windows(size)
            .map(|w| serde_json::Value::Array(w.to_vec()))
            .collect();
        Ok(arr(&windows))
    }
}

#[derive(Clone, Copy)]
pub struct Transpose;
impl Function for Transpose {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let array = require_array(args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?)?;
        if array.is_empty() {
            return Ok(arr(&[]));
        }
        let rows: Vec<&[serde_json::Value]> = array
            .iter()
            .map(|item| {
                item.as_array()
                    .map(|a| a.as_slice())
                    .unwrap_or(std::slice::from_ref(item))
            })
            .collect();
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if max_cols == 0 {
            return Ok(arr(&[]));
        }
        let mut transposed = Vec::with_capacity(max_cols);
        for col in 0..max_cols {
            let column: Vec<serde_json::Value> =
                rows.iter().filter_map(|r| r.get(col).cloned()).collect();
            transposed.push(serde_json::Value::Array(column));
        }
        Ok(arr(&transposed))
    }
}

// ── insert_at / remove_at ──────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct InsertAt;
impl Function for InsertAt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
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
        let index = (index as usize).min(array.len());
        let value = args[2].inner().clone();
        let mut result = Vec::with_capacity(array.len() + 1);
        result.extend_from_slice(&array[..index]);
        result.push(value);
        result.extend_from_slice(&array[index..]);
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct RemoveAt;
impl Function for RemoveAt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
        let index = index as usize;
        if index >= array.len() {
            return Ok(arr(array));
        }
        let mut result = Vec::with_capacity(array.len() - 1);
        result.extend_from_slice(&array[..index]);
        result.extend_from_slice(&array[index + 1..]);
        Ok(arr(&result))
    }
}
