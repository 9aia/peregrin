use ac_runtime::RuntimeValue;
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
            _ => None,
        }
    }
}

fn require_array(arg: &RuntimeValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn require_one_array(args: &[RuntimeValue]) -> Result<&[serde_json::Value], Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_array(arg)
}

fn require_two_arrays(
    args: &[RuntimeValue],
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

fn arr(items: &[serde_json::Value]) -> RuntimeValue {
    RuntimeValue::new(serde_json::Value::Array(items.to_vec()))
}

fn num(value: f64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(value))
}

fn b(value: bool) -> RuntimeValue {
    RuntimeValue::new(value)
}

macro_rules! array_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
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
        Some(v) => RuntimeValue::new(v.clone()),
        None => RuntimeValue::new(serde_json::Value::Null),
    }
});
array_unary_fn!(Last, |input: &[serde_json::Value]| {
    match input.last() {
        Some(v) => RuntimeValue::new(v.clone()),
        None => RuntimeValue::new(serde_json::Value::Null),
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
        None => RuntimeValue::new(serde_json::Value::Null),
    }
});
array_unary_fn!(Max, |input: &[serde_json::Value]| {
    let numbers: Vec<f64> = input.iter().filter_map(|v| v.as_f64()).collect();
    match numbers.iter().copied().max_by(|a, c| a.total_cmp(c)) {
        Some(m) => num(m),
        None => RuntimeValue::new(serde_json::Value::Null),
    }
});

#[derive(Clone, Copy)]
pub struct Concat;
impl Function for Concat {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let combined: Vec<serde_json::Value> = a.iter().chain(b.iter()).cloned().collect();
        Ok(arr(&combined))
    }
}

#[derive(Clone, Copy)]
pub struct Contains;
impl Function for Contains {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
        })? as usize;
        let end = args[2].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as usize;
        let end = end.min(array.len());
        let start = start.min(end);
        Ok(arr(&array[start..end]))
    }
}

#[derive(Clone, Copy)]
pub struct Sort;
impl Function for Sort {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
