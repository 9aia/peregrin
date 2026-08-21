use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

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
