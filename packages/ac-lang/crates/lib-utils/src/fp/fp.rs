use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

/// The `fp` module — functional programming utility functions.
///
/// Provides functions callable as `fp.identity(x)`, `fp.default(x, y)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct FpModule;

impl Module for FpModule {
    fn name(&self) -> &str {
        "fp"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "identity" => Some(&Identity),
            "default" => Some(&Default),
            "coalesce" => Some(&Coalesce),
            "is_null" => Some(&IsNull),
            "is_bool" => Some(&IsBool),
            "is_number" => Some(&IsNumber),
            "is_string" => Some(&IsString),
            "is_array" => Some(&IsArray),
            "is_object" => Some(&IsObject),
            "type_of" => Some(&TypeOf),
            "is_empty" => Some(&IsEmpty),
            "is_truthy" => Some(&IsTruthy),
            "range" => Some(&Range),
            "zip" => Some(&Zip),
            "chunk" => Some(&Chunk),
            "keys" => Some(&Keys),
            "values" => Some(&Values),
            "entries" => Some(&Entries),
            "pick" => Some(&Pick),
            "omit" => Some(&Omit),
            "merge" => Some(&Merge),
            "map" => Some(&Map),
            "filter" => Some(&Filter),
            "reduce" => Some(&Reduce),
            "pipe" => Some(&Pipe),
            "flow" => Some(&Flow),
            "compose" => Some(&Compose),
            "fanout" => Some(&Fanout),
            "choice" => Some(&Choice),
            "flat_map" => Some(&FlatMap),
            "find" => Some(&Find),
            "some" => Some(&Any),
            "every" => Some(&Every),
            "group_by" => Some(&GroupBy),
            "partition" => Some(&Partition),
            "sort_by" => Some(&SortBy),
            "apply" => Some(&Apply),
            "tap" => Some(&Tap),
            _ => None,
        }
    }
}

fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })
}

fn require_two_args(args: &[NonFunctionValue]) -> Result<(&NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    Ok((&args[0], &args[1]))
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn s(value: &str) -> NonFunctionValue {
    NonFunctionValue::new(value.to_string())
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn require_object(
    arg: &NonFunctionValue,
) -> Result<&serde_json::Map<String, serde_json::Value>, Error> {
    arg.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object",
    })
}

// ── identity ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Identity;
impl Function for Identity {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        Ok(arg.clone())
    }
}

// ── default ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Default;
impl Function for Default {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (value, fallback) = require_two_args(args)?;
        if value.is_null() {
            Ok(fallback.clone())
        } else {
            Ok(value.clone())
        }
    }
}

// ── coalesce ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Coalesce;
impl Function for Coalesce {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        for arg in args {
            if !arg.is_null() {
                return Ok(arg.clone());
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Null))
    }
}

// ── type predicates ────────────────────────────────────────────────

macro_rules! type_predicate {
    ($struct_name:ident, $predicate:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut Context,
            ) -> Result<NonFunctionValue, Error> {
                let arg = require_one_arg(args)?;
                Ok(b($predicate(arg)))
            }
        }
    };
}

type_predicate!(IsNull, |arg: &NonFunctionValue| arg.is_null());
type_predicate!(IsBool, |arg: &NonFunctionValue| arg.is_bool());
type_predicate!(IsNumber, |arg: &NonFunctionValue| arg.is_number());
type_predicate!(IsString, |arg: &NonFunctionValue| arg.is_string());
type_predicate!(IsArray, |arg: &NonFunctionValue| arg.is_array());
type_predicate!(IsObject, |arg: &NonFunctionValue| arg.is_object());

// ── type_of ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct TypeOf;
impl Function for TypeOf {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let type_name = if arg.is_null() {
            "null"
        } else if arg.is_bool() {
            "boolean"
        } else if arg.is_number() {
            "number"
        } else if arg.is_string() {
            "string"
        } else if arg.is_array() {
            "array"
        } else if arg.is_object() {
            "object"
        } else {
            "unknown"
        };
        Ok(s(type_name))
    }
}

// ── is_empty ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct IsEmpty;
impl Function for IsEmpty {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let empty = if arg.is_null() {
            true
        } else if let Some(str_val) = arg.as_str() {
            str_val.is_empty()
        } else if let Some(arr) = arg.as_array() {
            arr.is_empty()
        } else if let Some(obj) = arg.as_object() {
            obj.is_empty()
        } else {
            false
        };
        Ok(b(empty))
    }
}

// ── is_truthy ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct IsTruthy;
impl Function for IsTruthy {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let truthy = if let Some(b_val) = arg.as_bool() {
            b_val
        } else if let Some(n) = arg.as_number() {
            n != 0.0
        } else if let Some(str_val) = arg.as_str() {
            !str_val.is_empty()
        } else if arg.is_null() {
            false
        } else if let Some(arr) = arg.as_array() {
            !arr.is_empty()
        } else if let Some(obj) = arg.as_object() {
            !obj.is_empty()
        } else {
            true
        };
        Ok(b(truthy))
    }
}

// ── range ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Range;
impl Function for Range {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

// ── keys / values / entries ────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Keys;
impl Function for Keys {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let keys: Vec<serde_json::Value> = obj
            .keys()
            .map(|k| serde_json::Value::String(k.clone()))
            .collect();
        Ok(arr(&keys))
    }
}

#[derive(Clone, Copy)]
pub struct Values;
impl Function for Values {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let values: Vec<serde_json::Value> = obj.values().cloned().collect();
        Ok(arr(&values))
    }
}

#[derive(Clone, Copy)]
pub struct Entries;
impl Function for Entries {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let entries: Vec<serde_json::Value> = obj
            .iter()
            .map(|(k, v)| serde_json::json!([serde_json::Value::String(k.clone()), v.clone()]))
            .collect();
        Ok(arr(&entries))
    }
}

// ── pick / omit ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pick;
impl Function for Pick {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let obj = require_object(&args[0])?;
        let keys = require_array(&args[1])?;
        let mut result = serde_json::Map::new();
        for key_val in keys {
            if let Some(key) = key_val.as_str()
                && let Some(v) = obj.get(key)
            {
                result.insert(key.to_string(), v.clone());
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

#[derive(Clone, Copy)]
pub struct Omit;
impl Function for Omit {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let obj = require_object(&args[0])?;
        let keys = require_array(&args[1])?;
        let omit_set: std::collections::HashSet<&str> =
            keys.iter().filter_map(|v| v.as_str()).collect();
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            if !omit_set.contains(k.as_str()) {
                result.insert(k.clone(), v.clone());
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

// ── merge ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Merge;
impl Function for Merge {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let a = require_object(&args[0])?;
        let b = require_object(&args[1])?;
        let mut result = a.clone();
        for (k, v) in b {
            result.insert(k.clone(), v.clone());
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

// ── higher-order functions ─────────────────────────────────────────
//
// These functions take a FunctionValue (object with a `body` field) and
// an array. They use `ctx.eval_expr` to evaluate the function body with
// the element bound to the first parameter name.
//
// Requires an Eval callback on the Context. Returns PrimitiveError if
// no callback is set.

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

fn bind_param(fn_val: &FunctionValue, value: &serde_json::Value) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| vec![(name.clone(), NonFunctionValue::new(value.clone()))])
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
pub struct Map;
impl Function for Map {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Filter;
impl Function for Filter {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Reduce;
impl Function for Reduce {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

// ── pipe / flow / compose ──────────────────────────────────────────
//
// pipe(value, f, g, h) — immediate: evaluates h(g(f(value))) and returns
//   the result. Uses ctx.eval_expr for each step.
//
// flow(f, g, h) — deferred: returns a FunctionValue whose body inlines
//   the chain left-to-right: h(g(f(x))). The first param of each function
//   is substituted with the previous function's body.
//
// compose(f, g, h) — deferred: returns a FunctionValue whose body inlines
//   the chain right-to-left: f(g(h(x))). Same as flow but reversed order.

fn require_functions(args: &[NonFunctionValue]) -> Result<Vec<FunctionValue>, Error> {
    if args.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    args.iter().map(require_function_value).collect()
}

fn substitute_param(body: &str, param: &str, replacement: &str) -> String {
    let wrapped = format!("({replacement})");
    let mut result = String::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < body.len() {
        if body[i..].starts_with(param) {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after = i + param.len();
            let after_ok = after >= body.len() || !is_ident_char(bytes[after]);
            if before_ok && after_ok {
                result.push_str(&wrapped);
                i = after;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn inline_chain(fns: &[FunctionValue]) -> Result<FunctionValue, Error> {
    if fns.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    let first_fn = &fns[0];
    let input_param = first_fn
        .params
        .first()
        .map(|(name, _)| name.as_str())
        .unwrap_or("x");

    let mut body = first_fn.body.clone();
    for fn_val in &fns[1..] {
        let param = fn_val
            .params
            .first()
            .map(|(name, _)| name.as_str())
            .unwrap_or("x");
        body = substitute_param(&fn_val.body, param, &body);
    }

    Ok(FunctionValue {
        params: vec![(input_param.to_string(), serde_json::Value::Null)],
        return_schema: None,
        body,
    })
}

#[derive(Clone, Copy)]
pub struct Pipe;
impl Function for Pipe {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let mut value = args[0].clone();
        for fn_arg in &args[1..] {
            let fn_val = require_function_value(fn_arg)?;
            let vars = bind_param(&fn_val, &value.into_inner());
            value = ctx.eval_expr(&fn_val.body, &vars)?;
        }
        Ok(value)
    }
}

#[derive(Clone, Copy)]
pub struct Flow;
impl Function for Flow {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let fns = require_functions(args)?;
        let composed = inline_chain(&fns)?;
        Ok(NonFunctionValue::new_function(composed))
    }
}

#[derive(Clone, Copy)]
pub struct Compose;
impl Function for Compose {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let fns = require_functions(args)?;
        let reversed: Vec<FunctionValue> = fns.into_iter().rev().collect();
        let composed = inline_chain(&reversed)?;
        Ok(NonFunctionValue::new_function(composed))
    }
}

// ── fanout / choice ────────────────────────────────────────────────
//
// fanout(value, f, g, h) — immediate arrow combinator (&&&): applies
//   each function to the same value, returns [f(value), g(value), …].
//   Requires ctx.eval_expr.
//
// choice(value, predicate, f, g) — immediate arrow combinator (|||):
//   evaluates predicate(value); if truthy returns f(value), else
//   returns g(value). Requires ctx.eval_expr.

#[derive(Clone, Copy)]
pub struct Fanout;
impl Function for Fanout {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let value = args[0].clone();
        let inner = value.into_inner();
        let mut results = Vec::with_capacity(args.len() - 1);
        for fn_arg in &args[1..] {
            let fn_val = require_function_value(fn_arg)?;
            let vars = bind_param(&fn_val, &inner);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            results.push(result.into_inner());
        }
        Ok(arr(&results))
    }
}

#[derive(Clone, Copy)]
pub struct Choice;
impl Function for Choice {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 4 {
            return Err(Error::ArgumentCount {
                expected: 4,
                got: args.len(),
            });
        }
        let value = &args[0];
        let predicate = require_function_value(&args[1])?;
        let on_true = require_function_value(&args[2])?;
        let on_false = require_function_value(&args[3])?;

        let inner = value.clone().into_inner();
        let vars = bind_param(&predicate, &inner);
        let cond = ctx.eval_expr(&predicate.body, &vars)?;
        let branch = if is_truthy_value(&cond) { &on_true } else { &on_false };
        let vars = bind_param(branch, &inner);
        Ok(ctx.eval_expr(&branch.body, &vars)?)
    }
}

// ── flat_map / find / some / every ─────────────────────────────────
//
// flat_map(fn, array) — map each element through fn, flatten results.
// find(fn, array) — first element where predicate is truthy, or null.
// some(fn, array) — true if any element matches predicate.
// every(fn, array) — true if all elements match predicate.

#[derive(Clone, Copy)]
pub struct FlatMap;
impl Function for FlatMap {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Find;
impl Function for Find {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Any;
impl Function for Any {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

// ── group_by / partition / sort_by ─────────────────────────────────
//
// group_by(fn, array) — group elements by key function, returns object
//   mapping string keys to arrays of elements.
// partition(fn, array) — returns [matching, non-matching].
// sort_by(fn, array) — sort by numeric key extracted by fn (ascending).

#[derive(Clone, Copy)]
pub struct GroupBy;
impl Function for GroupBy {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct Partition;
impl Function for Partition {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

#[derive(Clone, Copy)]
pub struct SortBy;
impl Function for SortBy {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
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

// ── apply / tap ────────────────────────────────────────────────────
//
// apply(fn, args_array) — call a function value with multiple arguments
//   bound to all its params. Requires ctx.eval_expr.
// tap(fn, value) — call fn(value), return value unchanged.

fn bind_all_params(
    fn_val: &FunctionValue,
    args: &[serde_json::Value],
) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let val = args.get(i).cloned().unwrap_or(serde_json::Value::Null);
            (name.clone(), NonFunctionValue::new(val))
        })
        .collect()
}

#[derive(Clone, Copy)]
pub struct Apply;
impl Function for Apply {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let call_args = require_array(&args[1])?;
        let vars = bind_all_params(&fn_val, call_args);
        Ok(ctx.eval_expr(&fn_val.body, &vars)?)
    }
}

#[derive(Clone, Copy)]
pub struct Tap;
impl Function for Tap {
    fn call(&self, args: &[NonFunctionValue], ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let value = &args[1];
        let inner = value.clone().into_inner();
        let vars = bind_param(&fn_val, &inner);
        let _ = ctx.eval_expr(&fn_val.body, &vars)?;
        Ok(value.clone())
    }
}
