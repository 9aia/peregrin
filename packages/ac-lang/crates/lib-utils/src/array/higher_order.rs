use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_vm::{Context, Error, Function};

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })
}

fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

fn require_function_and_array(
    args: &[NonFunctionValue],
) -> Result<(FunctionValue, &[serde_json::Value]), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let fn_val = require_function_value(&args[0])?;
    let array = require_array(&args[1])?;
    Ok((fn_val, array))
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

fn key_to_string(value: &NonFunctionValue) -> String {
    if let Some(s) = value.as_str() {
        s.to_string()
    } else if let Some(n) = value.as_number() {
        if n == n.trunc() {
            format!("{n:.0}")
        } else {
            n.to_string()
        }
    } else if let Some(b) = value.as_bool() {
        b.to_string()
    } else if value.is_null() {
        "null".to_string()
    } else {
        serde_json::to_string(&value.clone().into_inner()).unwrap_or_else(|_| "unknown".to_string())
    }
}

// ── range ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Range;
impl Function for Range {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let start = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        let end = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        let step = if args.len() >= 3 {
            args[2].as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })?
        } else {
            1.0
        };
        if step == 0.0 {
            return Err(Error::TypeMismatch {
                expected: "non-zero step",
                got: "zero step",
            });
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
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
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
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let input = require_array(&args[0])?;
        let size = args[1].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })?;
        if size <= 0.0 {
            return Err(Error::TypeMismatch {
                expected: "positive chunk size",
                got: "non-positive chunk size",
            });
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
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut results = Vec::with_capacity(array.len());
        for item in array {
            let vars = bind_param(&fn_val, item);
            let mapped = ctx.eval_expr(&fn_val.body, &vars)?;
            results.push(mapped.into_inner());
        }
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
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut results = Vec::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let keep = is_truthy_value(&result);
            if keep {
                results.push(item.clone());
            }
        }
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
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let array = require_array(&args[1])?;
        let mut acc = args[2].clone();

        let param_names: Vec<String> = fn_val.params.iter().map(|(name, _)| name.clone()).collect();
        let acc_name = param_names
            .first()
            .cloned()
            .unwrap_or_else(|| "acc".to_string());
        let item_name = param_names
            .get(1)
            .cloned()
            .unwrap_or_else(|| "x".to_string());

        for item in array {
            let vars = vec![
                (acc_name.clone(), acc.clone()),
                (item_name.clone(), NonFunctionValue::new(item.clone())),
            ];
            acc = ctx.eval_expr(&fn_val.body, &vars)?;
        }
        Ok(acc)
    }
}

// ── flat_map ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FlatMap;
impl Function for FlatMap {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut results = Vec::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let mapped = ctx.eval_expr(&fn_val.body, &vars)?;
            if let Some(sub) = mapped.as_array() {
                results.extend(sub.iter().cloned());
            } else {
                results.push(mapped.into_inner());
            }
        }
        Ok(arr(&results))
    }
}

// ── find ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Find;
impl Function for Find {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        for item in array {
            let vars = bind_param(&fn_val, item);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result) {
                return Ok(NonFunctionValue::new(item.clone()));
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Null))
    }
}

// ── some / every ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct SomeFn;
impl Function for SomeFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        for item in array {
            let vars = bind_param(&fn_val, item);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result) {
                return Ok(b(true));
            }
        }
        Ok(b(false))
    }
}

#[derive(Clone, Copy)]
pub struct Every;
impl Function for Every {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        for item in array {
            let vars = bind_param(&fn_val, item);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            if !is_truthy_value(&result) {
                return Ok(b(false));
            }
        }
        Ok(b(true))
    }
}

// ── group_by ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct GroupBy;
impl Function for GroupBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut groups: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
            std::collections::BTreeMap::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
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

// ── partition ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Partition;
impl Function for Partition {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut matching = Vec::new();
        let mut rest = Vec::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&result) {
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

// ── sort_by / sort_by_desc ─────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct SortBy;
impl Function for SortBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut keyed: Vec<(f64, serde_json::Value)> = Vec::with_capacity(array.len());
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
            let num = key.as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })?;
            keyed.push((num, item.clone()));
        }
        keyed.sort_by(|a, c| a.0.total_cmp(&c.0));
        let sorted: Vec<serde_json::Value> = keyed.into_iter().map(|(_, v)| v).collect();
        Ok(arr(&sorted))
    }
}

#[derive(Clone, Copy)]
pub struct SortByDesc;
impl Function for SortByDesc {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut keyed: Vec<(f64, serde_json::Value)> = Vec::with_capacity(array.len());
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
            let num = key.as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })?;
            keyed.push((num, item.clone()));
        }
        keyed.sort_by(|a, c| c.0.total_cmp(&a.0));
        let sorted: Vec<serde_json::Value> = keyed.into_iter().map(|(_, v)| v).collect();
        Ok(arr(&sorted))
    }
}

// ── compact ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Compact;
impl Function for Compact {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_arg(args)?;
        let array = require_array(input)?;
        let result: Vec<serde_json::Value> =
            array.iter().filter(|v| !v.is_null()).cloned().collect();
        Ok(arr(&result))
    }
}

// ── pluck ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pluck;
impl Function for Pluck {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let array = require_array(&args[0])?;
        let key = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
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

// ── min_by / max_by ────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct MinBy;
impl Function for MinBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut best: Option<(f64, &serde_json::Value)> = None;
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
            let num = key.as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })?;
            match &best {
                None => best = Some((num, item)),
                Some((bn, _)) if num < *bn => best = Some((num, item)),
                _ => {}
            }
        }
        match best {
            Some((_, v)) => Ok(NonFunctionValue::new(v.clone())),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct MaxBy;
impl Function for MaxBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut best: Option<(f64, &serde_json::Value)> = None;
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
            let num = key.as_number().ok_or(Error::TypeMismatch {
                expected: "number",
                got: "non-number",
            })?;
            match &best {
                None => best = Some((num, item)),
                Some((bn, _)) if num > *bn => best = Some((num, item)),
                _ => {}
            }
        }
        match best {
            Some((_, v)) => Ok(NonFunctionValue::new(v.clone())),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

// ── count_by ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct CountBy;
impl Function for CountBy {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_val, array) = require_function_and_array(args)?;
        let mut counts: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
        for item in array {
            let vars = bind_param(&fn_val, item);
            let key = ctx.eval_expr(&fn_val.body, &vars)?;
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
