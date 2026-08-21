use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

use super::higher_order::{
    Chunk, Compact, CountBy, Filter, Find, FlatMap, GroupBy, Map, MaxBy, MinBy,
    Partition, Pluck, Range, Reduce, SomeFn, Every, SortBy, SortByDesc, Zip,
};

/// The `array` module — array utility functions.
///
/// Provides functions callable as `array.length(a)`, `array.reverse(a)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct ArrayModule;

impl Module for ArrayModule {
    fn name(&self) -> &str {
        "array"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "length" => Some(&Length),
            "reverse" => Some(&Reverse),
            "concat" => Some(&Concat),
            "contains" => Some(&Contains),
            "index_of" => Some(&IndexOf),
            "slice" => Some(&Slice),
            "first" => Some(&First),
            "last" => Some(&Last),
            "sort" => Some(&Sort),
            "flatten" => Some(&Flatten),
            "unique" => Some(&Unique),
            "sum" => Some(&Sum),
            "avg" => Some(&Avg),
            "min" => Some(&Min),
            "max" => Some(&Max),
            "at" => Some(&At),
            "push" => Some(&Push),
            "fill" => Some(&Fill),
            "difference" => Some(&Difference),
            "intersection" => Some(&Intersection),
            "union" => Some(&Union),
            "deep_flatten" => Some(&DeepFlatten),
            "pop" => Some(&Pop),
            "shift" => Some(&Shift),
            "take" => Some(&Take),
            "drop" => Some(&Drop),
            "enumerate" => Some(&Enumerate),
            "unzip" => Some(&Unzip),
            "intersperse" => Some(&Intersperse),
            "intercalate" => Some(&Intercalate),
            "symmetric_difference" => Some(&SymmetricDifference),
            "dedupe" => Some(&Dedupe),
            "prepend" => Some(&Prepend),
            "without" => Some(&Without),
            "take_while" => Some(&TakeWhile),
            "drop_while" => Some(&DropWhile),
            "scan" => Some(&Scan),
            "foldr" => Some(&Foldr),
            "unique_by" => Some(&UniqueBy),
            "find_index" => Some(&FindIndex),
            "find_last" => Some(&FindLast),
            "find_last_index" => Some(&FindLastIndex),
            // higher-order (moved from fp module)
            "range" => Some(&Range),
            "zip" => Some(&Zip),
            "chunk" => Some(&Chunk),
            "map" => Some(&Map),
            "filter" => Some(&Filter),
            "reduce" => Some(&Reduce),
            "flat_map" => Some(&FlatMap),
            "find" => Some(&Find),
            "some" => Some(&SomeFn),
            "every" => Some(&Every),
            "group_by" => Some(&GroupBy),
            "partition" => Some(&Partition),
            "sort_by" => Some(&SortBy),
            "sort_by_desc" => Some(&SortByDesc),
            "compact" => Some(&Compact),
            "pluck" => Some(&Pluck),
            "min_by" => Some(&MinBy),
            "max_by" => Some(&MaxBy),
            "count_by" => Some(&CountBy),
            _ => None,
        }
    }
}

fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn require_one_array(args: &[NonFunctionValue]) -> Result<&[serde_json::Value], Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_array(arg)
}

fn require_two_arrays(
    args: &[NonFunctionValue],
) -> Result<(&[serde_json::Value], &[serde_json::Value]), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_array(&args[0])?;
    let b = require_array(&args[1])?;
    Ok((a, b))
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

fn bind_param(fn_val: &FunctionValue, value: &serde_json::Value) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| vec![(name.clone(), NonFunctionValue::new(value.clone()))])
        .unwrap_or_default()
}

fn bind_two_params(
    fn_val: &FunctionValue,
    acc: &serde_json::Value,
    item: &serde_json::Value,
) -> Vec<(String, NonFunctionValue)> {
    let param_names: Vec<&String> = fn_val.params.iter().map(|(name, _)| name).collect();
    let acc_name = param_names.first().map(|s| s.as_str()).unwrap_or("acc");
    let item_name = param_names.get(1).map(|s| s.as_str()).unwrap_or("x");
    vec![
        (acc_name.to_string(), NonFunctionValue::new(acc.clone())),
        (item_name.to_string(), NonFunctionValue::new(item.clone())),
    ]
}

fn is_truthy_value(value: &NonFunctionValue) -> bool {
    if let Some(b) = value.as_bool() {
        b
    } else if let Some(n) = value.as_number() {
        n != 0.0
    } else if let Some(s) = value.as_str() {
        !s.is_empty()
    } else if value.is_null() {
        false
    } else if let Some(arr) = value.as_array() {
        !arr.is_empty()
    } else if let Some(obj) = value.as_object() {
        !obj.is_empty()
    } else {
        true
    }
}

macro_rules! array_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut Context,
            ) -> Result<NonFunctionValue, Error> {
                let input = require_one_array(args)?;
                Ok($body(input))
            }
        }
    };
}

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
array_unary_fn!(Sum, |input: &[serde_json::Value]| {
    let total: f64 = input.iter().filter_map(|v| v.as_f64()).sum();
    num(total)
});
array_unary_fn!(Avg, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    if numbers.is_empty() {
        num(0.0)
    } else {
        num(numbers.iter().sum::<f64>() / numbers.len() as f64)
    }
});
array_unary_fn!(Min, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    match numbers.iter().copied().min_by(|a, c| a.total_cmp(c)) {
        Some(m) => num(m),
        None => NonFunctionValue::new(serde_json::Value::Null),
    }
});
array_unary_fn!(Max, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    match numbers.iter().copied().max_by(|a, c| a.total_cmp(c)) {
        Some(m) => num(m),
        None => NonFunctionValue::new(serde_json::Value::Null),
    }
});

#[derive(Clone, Copy)]
pub struct Concat;
impl Function for Concat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let combined: Vec<serde_json::Value> = a.iter().chain(b.iter()).cloned().collect();
        Ok(arr(&combined))
    }
}

#[derive(Clone, Copy)]
pub struct Contains;
impl Function for Contains {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
impl Function for IndexOf {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
impl Function for Slice {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
impl Function for Sort {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
pub struct Unique;
impl Function for Unique {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        let mut seen: Vec<serde_json::Value> = Vec::new();
        for item in input {
            if !seen.contains(item) {
                seen.push(item.clone());
            }
        }
        Ok(arr(&seen))
    }
}

#[derive(Clone, Copy)]
pub struct At;
impl Function for At {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
impl Function for Push {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
impl Function for Fill {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
pub struct Difference;
impl Function for Difference {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let result: Vec<serde_json::Value> = a
            .iter()
            .filter(|item| !b.contains(item))
            .cloned()
            .collect();
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct Intersection;
impl Function for Intersection {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let result: Vec<serde_json::Value> = a
            .iter()
            .filter(|item| b.contains(item))
            .cloned()
            .collect();
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct Union;
impl Function for Union {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let mut result: Vec<serde_json::Value> = a.to_vec();
        for item in b {
            if !result.contains(item) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

#[derive(Clone, Copy)]
pub struct DeepFlatten;
impl Function for DeepFlatten {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
pub struct Pop;
impl Function for Pop {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let array = require_one_array(args)?;
        if array.is_empty() {
            return Ok(arr(&[]));
        }
        Ok(arr(&array[..array.len() - 1]))
    }
}

#[derive(Clone, Copy)]
pub struct Shift;
impl Function for Shift {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let array = require_one_array(args)?;
        if array.is_empty() {
            return Ok(arr(&[]));
        }
        Ok(arr(&array[1..]))
    }
}

// ── take / drop ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Take;
impl Function for Take {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

// ── symmetric_difference / dedupe / prepend / without ──────────────

#[derive(Clone, Copy)]
pub struct SymmetricDifference;
impl Function for SymmetricDifference {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let mut result: Vec<serde_json::Value> = a
            .iter()
            .filter(|item| !b.contains(item))
            .cloned()
            .collect();
        for item in b {
            if !a.contains(item) && !result.contains(item) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Without;
impl Function for Without {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let array = require_array(&args[0])?;
        let excludes: Vec<&serde_json::Value> = args[1..].iter().map(|v| v.inner()).collect();
        let result: Vec<serde_json::Value> = array
            .iter()
            .filter(|item| !excludes.contains(item))
            .cloned()
            .collect();
        Ok(arr(&result))
    }
}

// ── take_while / drop_while ────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct TakeWhile;
impl Function for TakeWhile {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
            let vars = bind_param(&fn_val, item);
            let result_val = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result_val) {
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
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
            let vars = bind_param(&fn_val, item);
            let result_val = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result_val) {
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
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
            let vars = bind_two_params(&fn_val, &acc, item);
            let result_val = ctx.eval_expr(&fn_val.body, &vars)?;
            acc = result_val.into_inner();
            results.push(acc.clone());
        }
        Ok(arr(&results))
    }
}

#[derive(Clone, Copy)]
pub struct Foldr;
impl Function for Foldr {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
            let vars = bind_two_params(&fn_val, acc.inner(), item);
            acc = ctx.eval_expr(&fn_val.body, &vars)?;
        }
        Ok(acc)
    }
}

// ── unique_by ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct UniqueBy;
impl Function for UniqueBy {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let mut seen: Vec<serde_json::Value> = Vec::new();
        let mut result = Vec::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
            let key_inner = key.into_inner();
            if !seen.contains(&key_inner) {
                seen.push(key_inner);
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── find_index / find_last / find_last_index ───────────────────────

#[derive(Clone, Copy)]
pub struct FindIndex;
impl Function for FindIndex {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        for (i, item) in array.iter().enumerate() {
            let vars = bind_param(&fn_val, item);
            let result_val = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result_val) {
                return Ok(num(i as f64));
            }
        }
        Ok(num(-1.0))
    }
}

#[derive(Clone, Copy)]
pub struct FindLast;
impl Function for FindLast {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        for item in array.iter().rev() {
            let vars = bind_param(&fn_val, item);
            let result_val = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result_val) {
                return Ok(NonFunctionValue::new(item.clone()));
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Null))
    }
}

#[derive(Clone, Copy)]
pub struct FindLastIndex;
impl Function for FindLastIndex {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        for (i, item) in array.iter().enumerate().rev() {
            let vars = bind_param(&fn_val, item);
            let result_val = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result_val) {
                return Ok(num(i as f64));
            }
        }
        Ok(num(-1.0))
    }
}
