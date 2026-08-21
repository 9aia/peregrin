use std::collections::HashSet;

use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

/// The `set` module — set-theoretic operations on arrays.
///
/// All functions accept JSON arrays and return JSON arrays with
/// duplicates removed (set-correct semantics). Internally uses
/// `HashSet` for O(n) lookup.
///
/// Callable as `set.union(a, b)`, `set.intersection(a, b)`, etc.
#[derive(Clone, Copy)]
pub struct SetModule;

impl Module for SetModule {
    fn name(&self) -> &str {
        "set"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "unique" => Some(&Unique),
            "union" => Some(&Union),
            "intersection" => Some(&Intersection),
            "difference" => Some(&Difference),
            "symmetric_difference" => Some(&SymmetricDifference),
            "without" => Some(&Without),
            "unique_by" => Some(&UniqueBy),
            _ => None,
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────

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

fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

fn bind_param(
    fn_val: &FunctionValue,
    value: &serde_json::Value,
) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| vec![(name.clone(), NonFunctionValue::new(value.clone()))])
        .unwrap_or_default()
}

/// Serialize a `serde_json::Value` to a string for use as a `HashSet` key.
/// This handles all JSON types (numbers, strings, bools, null, arrays, objects).
fn value_key(value: &serde_json::Value) -> String {
    // For primitives, use a type-prefixed representation to avoid
    // collisions (e.g. the string "1" vs the number 1).
    match value {
        serde_json::Value::Null => "n:".to_string(),
        serde_json::Value::Bool(b) => format!("b:{b}"),
        serde_json::Value::Number(n) => format!("n:{n}"),
        serde_json::Value::String(s) => format!("s:{s}"),
        other => serde_json::to_string(other).unwrap_or_else(|_| "?".to_string()),
    }
}

/// Deduplicate a slice while preserving first-occurrence order.
fn dedup_ordered(items: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        let key = value_key(item);
        if seen.insert(key) {
            result.push(item.clone());
        }
    }
    result
}

// ── unique ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Unique;
impl Function for Unique {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_array(args)?;
        Ok(arr(&dedup_ordered(input)))
    }
}

// ── union ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Union;
impl Function for Union {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::with_capacity(a.len() + b.len());
        for item in a.iter().chain(b.iter()) {
            let key = value_key(item);
            if seen.insert(key) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── intersection ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Intersection;
impl Function for Intersection {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let b_set: HashSet<String> = b.iter().map(value_key).collect();
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();
        for item in a {
            let key = value_key(item);
            if b_set.contains(&key) && seen.insert(key) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── difference ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Difference;
impl Function for Difference {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let b_set: HashSet<String> = b.iter().map(value_key).collect();
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();
        for item in a {
            let key = value_key(item);
            if !b_set.contains(&key) && seen.insert(key) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── symmetric_difference ──────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct SymmetricDifference;
impl Function for SymmetricDifference {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_arrays(args)?;
        let a_set: HashSet<String> = a.iter().map(value_key).collect();
        let b_set: HashSet<String> = b.iter().map(value_key).collect();
        let mut result = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for item in a {
            let key = value_key(item);
            if !b_set.contains(&key) && seen.insert(key) {
                result.push(item.clone());
            }
        }
        for item in b {
            let key = value_key(item);
            if !a_set.contains(&key) && seen.insert(key) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── without ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Without;
impl Function for Without {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let array = require_array(&args[0])?;
        let excludes: HashSet<String> = args[1..].iter().map(|v| value_key(v.inner())).collect();
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();
        for item in array {
            let key = value_key(item);
            if !excludes.contains(&key) && seen.insert(key) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}

// ── unique_by ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct UniqueBy;
impl Function for UniqueBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let fn_val = require_function_value(&args[1])?;
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
            let key_str = value_key(&key.into_inner());
            if seen.insert(key_str) {
                result.push(item.clone());
            }
        }
        Ok(arr(&result))
    }
}
