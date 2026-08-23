use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function, GenericFunction};

use super::helpers::{
    arr, b, key_to_string, require_array, require_function_and_array, require_function_value,
    require_one_arg};
use crate::shared::{apply_fn, filter_iter, find_first, fold_iter, is_truthy_fn, map_iter};

// ── range ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Range;
impl Function for Range {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len()});
        }
        let start = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number"})?;
        let end = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number"})?;
        let step = if args.len() >= 3 {
            args[2].as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number"})?
        } else {
            1.0
        };
        if step == 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-zero step",
                got: "zero step"});
        }
        let mut values = Vec::new();
        let mut current = start;
        if step > 0.0 {
            while current < end {
                values.push(serde_json::json!(current));
                current += step;
            }
        } else {
            while current > end {
                values.push(serde_json::json!(current));
                current += step;
            }
        }
        Ok(arr(&values))
    }
}

// ── zip ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Zip;
impl Function for Zip {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len()});
        }
        let a = require_array(&args[0])?;
        let b = require_array(&args[1])?;
        let len = a.len().min(b.len());
        let pairs: Vec<serde_json::Value> = (0..len)
            .map(|i| serde_json::json!([a[i].clone(), b[i].clone()]))
            .collect();
        Ok(arr(&pairs))
    }
}

// ── chunk ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Chunk;
impl Function for Chunk {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len()});
        }
        let input = require_array(&args[0])?;
        let size = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number"})?;
        if size <= 0.0 {
            return Err(Error::TypeMismatch {
                expected: "positive chunk size",
                got: "non-positive chunk size"});
        }
        let size = size as usize;
        let chunks: Vec<serde_json::Value> = input
            .chunks(size)
            .map(|chunk| serde_json::Value::Array(chunk.to_vec()))
            .collect();
        Ok(arr(&chunks))
    }
}

// ── map ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Map;
impl Function for Map {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let results = map_iter(ctx, &fn_val, array.iter())?;
        Ok(arr(&results))
    }
}

impl GenericFunction for Map {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        // array.map delegates to the same logic regardless of type_args,
        // but the GenericFunction impl allows the VM to dispatch here
        // when type_args are present, enabling type-aware host extensions.
        let (fn_val, array) = require_function_and_array(args)?;
        let results = map_iter(ctx, &fn_val, array.iter())?;
        Ok(arr(&results))
    }
}

// ── filter ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Filter;
impl Function for Filter {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let results = filter_iter(ctx, &fn_val, array.iter())?;
        Ok(arr(&results))
    }
}

impl GenericFunction for Filter {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let results = filter_iter(ctx, &fn_val, array.iter())?;
        Ok(arr(&results))
    }
}

// ── reduce ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Reduce;
impl Function for Reduce {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len()});
        }
        let fn_val = require_function_value(&args[0])?;
        let array = require_array(&args[1])?;
        let init = args[2].inner().clone();
        let result = fold_iter(ctx, &fn_val, array.iter(), init)?;
        Ok(NonFunctionValue::new(result))
    }
}

impl GenericFunction for Reduce {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len()});
        }
        let fn_val = require_function_value(&args[0])?;
        let array = require_array(&args[1])?;
        let init = args[2].inner().clone();
        let result = fold_iter(ctx, &fn_val, array.iter(), init)?;
        Ok(NonFunctionValue::new(result))
    }
}

// ── flat_map ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FlatMap;
impl Function for FlatMap {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mapped = map_iter(ctx, &fn_val, array.iter())?;
        let mut results = Vec::new();
        for item in mapped {
            if let Some(sub) = item.as_array() {
                results.extend(sub.iter().cloned());
            } else {
                results.push(item);
            }
        }
        Ok(arr(&results))
    }
}

impl GenericFunction for FlatMap {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── find ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Find;
impl Function for Find {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let result = find_first(ctx, &fn_val, array.iter())?;
        match result {
            Some(v) => Ok(NonFunctionValue::new(v)),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null))}
    }
}

impl GenericFunction for Find {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── some / every ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct SomeFn;
impl Function for SomeFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        for item in array {
            if is_truthy_fn(ctx, &fn_val, item)? {
                return Ok(b(true));
            }
        }
        Ok(b(false))
    }
}

impl GenericFunction for SomeFn {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

#[derive(Clone, Copy)]
pub struct Every;
impl Function for Every {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        for item in array {
            if !is_truthy_fn(ctx, &fn_val, item)? {
                return Ok(b(false));
            }
        }
        Ok(b(true))
    }
}

impl GenericFunction for Every {
    fn call_with_types(
        &self,
        args: &[NonFunctionValue],
        _type_args: &[String],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── group_by ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct GroupBy;
impl Function for GroupBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut groups: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
            std::collections::BTreeMap::new();
        for item in array {
            let key = apply_fn(ctx, &fn_val, item)?;
            let key_str = key_to_string(&key);
            groups.entry(key_str).or_default().push(item.clone());
        }
        let mut result = serde_json::Map::new();
        for (k, v) in groups {
            result.insert(k, serde_json::Value::Array(v));
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

impl GenericFunction for GroupBy {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── partition ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Partition;
impl Function for Partition {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut matching = Vec::new();
        let mut rest = Vec::new();
        for item in array {
            if is_truthy_fn(ctx, &fn_val, item)? {
                matching.push(item.clone());
            } else {
                rest.push(item.clone());
            }
        }
        Ok(arr(&[
            serde_json::Value::Array(matching),
            serde_json::Value::Array(rest),
        ]))
    }
}

impl GenericFunction for Partition {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── sort_by / sort_by_desc ─────────────────────────────────────────

fn sort_by_impl(
    args: &[NonFunctionValue],
    ctx: &mut dyn Context,
    descending: bool,
) -> Result<NonFunctionValue, Error> {
    let (fn_val, array) = require_function_and_array(args)?;
    let mut keyed: Vec<(f64, serde_json::Value)> = Vec::with_capacity(array.len());
    for item in array {
        let key = apply_fn(ctx, &fn_val, item)?;
        let num = key.as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number"})?;
        keyed.push((num, item.clone()));
    }
    if descending {
        keyed.sort_by(|a, c| c.0.total_cmp(&a.0));
    } else {
        keyed.sort_by(|a, c| a.0.total_cmp(&c.0));
    }
    let sorted: Vec<serde_json::Value> = keyed.into_iter().map(|(_, v)| v).collect();
    Ok(arr(&sorted))
}

#[derive(Clone, Copy)]
pub struct SortBy;
impl Function for SortBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        sort_by_impl(args, ctx, false)
    }
}

impl GenericFunction for SortBy {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

#[derive(Clone, Copy)]
pub struct SortByDesc;
impl Function for SortByDesc {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        sort_by_impl(args, ctx, true)
    }
}

impl GenericFunction for SortByDesc {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── compact ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Compact;
impl Function for Compact {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_arg(args)?;
        let array = require_array(input)?;
        let result: Vec<serde_json::Value> =
            array.iter().filter(|v| !v.is_null()).cloned().collect();
        Ok(arr(&result))
    }
}

impl GenericFunction for Compact {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── pluck ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pluck;
impl Function for Pluck {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len()});
        }
        let array = require_array(&args[0])?;
        let key = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string"})?;
        let result: Vec<serde_json::Value> = array
            .iter()
            .map(|item| {
                item.as_object()
                    .and_then(|obj| obj.get(key))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            })
            .collect();
        Ok(arr(&result))
    }
}

impl GenericFunction for Pluck {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── min_by / max_by ────────────────────────────────────────────────

fn extremum_by_impl(
    args: &[NonFunctionValue],
    ctx: &mut dyn Context,
    pick_max: bool,
) -> Result<NonFunctionValue, Error> {
    let (fn_val, array) = require_function_and_array(args)?;
    let mut best: Option<(f64, &serde_json::Value)> = None;
    for item in array {
        let key = apply_fn(ctx, &fn_val, item)?;
        let num = key.as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number"})?;
        match &best {
            None => best = Some((num, item)),
            Some((bn, _)) if pick_max && num > *bn => best = Some((num, item)),
            Some((bn, _)) if !pick_max && num < *bn => best = Some((num, item)),
            _ => {}
        }
    }
    match best {
        Some((_, v)) => Ok(NonFunctionValue::new(v.clone())),
        None => Ok(NonFunctionValue::new(serde_json::Value::Null))}
}

#[derive(Clone, Copy)]
pub struct MinBy;
impl Function for MinBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        extremum_by_impl(args, ctx, false)
    }
}

impl GenericFunction for MinBy {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

#[derive(Clone, Copy)]
pub struct MaxBy;
impl Function for MaxBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        extremum_by_impl(args, ctx, true)
    }
}

impl GenericFunction for MaxBy {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}

// ── count_by ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct CountBy;
impl Function for CountBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut counts: std::collections::BTreeMap<String, f64> =
            std::collections::BTreeMap::new();
        for item in array {
            let key = apply_fn(ctx, &fn_val, item)?;
            let key_str = key_to_string(&key);
            *counts.entry(key_str).or_insert(0.0) += 1.0;
        }
        let mut result = serde_json::Map::new();
        for (k, v) in counts {
            result.insert(k, serde_json::json!(v));
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

impl GenericFunction for CountBy {
    fn call_with_types(&self, args: &[NonFunctionValue], _type_args: &[String], ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        self.call(args, ctx)
    }
}
