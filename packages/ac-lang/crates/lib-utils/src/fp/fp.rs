use ac_runtime::{FunctionValue, RuntimeValue};
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
            _ => None,
        }
    }
}

fn require_one_arg(args: &[RuntimeValue]) -> Result<&RuntimeValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })
}

fn require_two_args(args: &[RuntimeValue]) -> Result<(&RuntimeValue, &RuntimeValue), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    Ok((&args[0], &args[1]))
}

fn b(value: bool) -> RuntimeValue {
    RuntimeValue::new(value)
}

fn s(value: &str) -> RuntimeValue {
    RuntimeValue::new(value.to_string())
}

fn arr(items: &[serde_json::Value]) -> RuntimeValue {
    RuntimeValue::new(serde_json::Value::Array(items.to_vec()))
}

fn require_array<'a>(arg: &'a RuntimeValue) -> Result<&'a [serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn require_object<'a>(
    arg: &'a RuntimeValue,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, Error> {
    arg.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object",
    })
}

// ── identity ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Identity;
impl Function for Identity {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let arg = require_one_arg(args)?;
        Ok(arg.clone())
    }
}

// ── default ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Default;
impl Function for Default {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        for arg in args {
            if !arg.is_null() {
                return Ok(arg.clone());
            }
        }
        Ok(RuntimeValue::new(serde_json::Value::Null))
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
                args: &[RuntimeValue],
                _ctx: &mut Context,
            ) -> Result<RuntimeValue, Error> {
                let arg = require_one_arg(args)?;
                Ok(b($predicate(arg)))
            }
        }
    };
}

type_predicate!(IsNull, |arg: &RuntimeValue| arg.is_null());
type_predicate!(IsBool, |arg: &RuntimeValue| arg.is_bool());
type_predicate!(IsNumber, |arg: &RuntimeValue| arg.is_number());
type_predicate!(IsString, |arg: &RuntimeValue| arg.is_string());
type_predicate!(IsArray, |arg: &RuntimeValue| arg.is_array());
type_predicate!(IsObject, |arg: &RuntimeValue| arg.is_object());

// ── type_of ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct TypeOf;
impl Function for TypeOf {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let start = args[0]
            .as_number()
            .ok_or(Error::TypeMismatch { expected: "number", got: "non-number" })?;
        let end = args[1]
            .as_number()
            .ok_or(Error::TypeMismatch { expected: "number", got: "non-number" })?;
        let step = if args.len() >= 3 {
            args[2]
                .as_number()
                .ok_or(Error::TypeMismatch { expected: "number", got: "non-number" })?
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
            .map(|i| {
                serde_json::json!([a[i].clone(), b[i].clone()])
            })
            .collect();
        Ok(arr(&pairs))
    }
}

// ── chunk ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Chunk;
impl Function for Chunk {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let input = require_array(&args[0])?;
        let size = args[1]
            .as_number()
            .ok_or(Error::TypeMismatch { expected: "number", got: "non-number" })?;
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let values: Vec<serde_json::Value> = obj.values().cloned().collect();
        Ok(arr(&values))
    }
}

#[derive(Clone, Copy)]
pub struct Entries;
impl Function for Entries {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let entries: Vec<serde_json::Value> = obj
            .iter()
            .map(|(k, v)| {
                serde_json::json!([serde_json::Value::String(k.clone()), v.clone()])
            })
            .collect();
        Ok(arr(&entries))
    }
}

// ── pick / omit ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pick;
impl Function for Pick {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
            if let Some(key) = key_val.as_str() {
                if let Some(v) = obj.get(key) {
                    result.insert(key.to_string(), v.clone());
                }
            }
        }
        Ok(RuntimeValue::new(serde_json::Value::Object(result)))
    }
}

#[derive(Clone, Copy)]
pub struct Omit;
impl Function for Omit {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
        Ok(RuntimeValue::new(serde_json::Value::Object(result)))
    }
}

// ── merge ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Merge;
impl Function for Merge {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
        Ok(RuntimeValue::new(serde_json::Value::Object(result)))
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

fn require_function_value(arg: &RuntimeValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

fn require_function_and_array<'a>(
    args: &'a [RuntimeValue],
) -> Result<(FunctionValue, &'a [serde_json::Value]), Error> {
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

fn bind_param(fn_val: &FunctionValue, value: &serde_json::Value) -> Vec<(String, RuntimeValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| {
            vec![(name.clone(), RuntimeValue::new(value.clone()))]
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
pub struct Map;
impl Function for Map {
    fn call(&self, args: &[RuntimeValue], ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], ctx: &mut Context) -> Result<RuntimeValue, Error> {
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

fn is_truthy_value(value: &RuntimeValue) -> bool {
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
    fn call(&self, args: &[RuntimeValue], ctx: &mut Context) -> Result<RuntimeValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let array = require_array(&args[1])?;
        let mut acc = args[2].clone();

        let param_names: Vec<String> = fn_val
            .params
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let acc_name = param_names.first().cloned().unwrap_or_else(|| "acc".to_string());
        let item_name = param_names.get(1).cloned().unwrap_or_else(|| "x".to_string());

        for item in array {
            let vars = vec![
                (acc_name.clone(), acc.clone()),
                (item_name.clone(), RuntimeValue::new(item.clone())),
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

fn require_functions(args: &[RuntimeValue]) -> Result<Vec<FunctionValue>, Error> {
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
    fn call(&self, args: &[RuntimeValue], ctx: &mut Context) -> Result<RuntimeValue, Error> {
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
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let fns = require_functions(args)?;
        let composed = inline_chain(&fns)?;
        Ok(RuntimeValue::new_function(composed))
    }
}

#[derive(Clone, Copy)]
pub struct Compose;
impl Function for Compose {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let fns = require_functions(args)?;
        let reversed: Vec<FunctionValue> = fns.into_iter().rev().collect();
        let composed = inline_chain(&reversed)?;
        Ok(RuntimeValue::new_function(composed))
    }
}
