use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

/// The `obj` module — object utility functions.
///
/// Provides functions callable as `obj.keys(o)`, `obj.get(o, k)`,
/// `obj.set(o, k, v)`, `obj.has_key(o, k)`, `obj.map_values(fn, o)`,
/// `obj.deep_get(o, path)`, `obj.deep_merge(a, b)`, etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct ObjModule;

impl Module for ObjModule {
    fn name(&self) -> &str {
        "obj"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "keys" => Some(&Keys),
            "values" => Some(&Values),
            "entries" => Some(&Entries),
            "from_entries" => Some(&FromEntries),
            "pick" => Some(&Pick),
            "omit" => Some(&Omit),
            "merge" => Some(&Merge),
            "deep_get" => Some(&DeepGet),
            "deep_merge" => Some(&DeepMerge),
            "get" => Some(&Get),
            "get_or" => Some(&GetOr),
            "set" => Some(&Set),
            "update" => Some(&Update),
            "has_key" => Some(&HasKey),
            "map_values" => Some(&MapValues),
            "map_keys" => Some(&MapKeys),
            "filter_keys" => Some(&FilterKeys),
            "filter_values" => Some(&FilterValues),
            "invert" => Some(&Invert),
            "path_or" => Some(&PathOr),
            "set_path" => Some(&SetPath),
            "update_path" => Some(&UpdatePath),
            "size" => Some(&Size),
            "clone_deep" => Some(&CloneDeep),
            _ => None,
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────

fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })
}

fn require_two_args(
    args: &[NonFunctionValue],
) -> Result<(&NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    Ok((&args[0], &args[1]))
}

fn require_three_args(
    args: &[NonFunctionValue],
) -> Result<(&NonFunctionValue, &NonFunctionValue, &NonFunctionValue), Error> {
    if args.len() < 3 {
        return Err(Error::ArgumentCount {
            expected: 3,
            got: args.len(),
        });
    }
    Ok((&args[0], &args[1], &args[2]))
}

fn require_object(
    arg: &NonFunctionValue,
) -> Result<&serde_json::Map<String, serde_json::Value>, Error> {
    arg.as_object().ok_or(Error::TypeMismatch {
        expected: "object",
        got: "non-object",
    })
}

fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn require_string(arg: &NonFunctionValue) -> Result<String, Error> {
    arg.as_str()
        .map(|s| s.to_string())
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

fn obj_value(map: serde_json::Map<String, serde_json::Value>) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Object(map))
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
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

fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == f.trunc() {
                    format!("{f:.0}")
                } else {
                    f.to_string()
                }
            } else {
                n.to_string()
            }
        }
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "unknown".to_string()),
    }
}

// ── keys / values / entries ────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Keys;
impl Function for Keys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let values: Vec<serde_json::Value> = obj.values().cloned().collect();
        Ok(arr(&values))
    }
}

#[derive(Clone, Copy)]
pub struct Entries;
impl Function for Entries {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
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
        let a = require_object(&args[0])?;
        let b = require_object(&args[1])?;
        let mut result = a.clone();
        for (k, v) in b {
            result.insert(k.clone(), v.clone());
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

// ── deep_get ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct DeepGet;
impl Function for DeepGet {
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
        let path = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        let mut current = args[0].inner().clone();
        for segment in path.split('.') {
            if segment.is_empty() {
                continue;
            }
            current = match &current {
                serde_json::Value::Object(obj) => {
                    obj.get(segment).cloned().unwrap_or(serde_json::Value::Null)
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(idx) = segment.parse::<usize>() {
                        arr.get(idx).cloned().unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                _ => serde_json::Value::Null,
            };
            if current.is_null() {
                break;
            }
        }
        Ok(NonFunctionValue::new(current))
    }
}

// ── deep_merge ─────────────────────────────────────────────────────

fn deep_merge_values(a: &serde_json::Value, b: &serde_json::Value) -> serde_json::Value {
    match (a, b) {
        (serde_json::Value::Object(a_obj), serde_json::Value::Object(b_obj)) => {
            let mut result = a_obj.clone();
            for (k, v) in b_obj {
                if let Some(existing) = result.get(k) {
                    result.insert(k.clone(), deep_merge_values(existing, v));
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
            serde_json::Value::Object(result)
        }
        (_, b) => b.clone(),
    }
}

#[derive(Clone, Copy)]
pub struct DeepMerge;
impl Function for DeepMerge {
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
        let a = require_object(&args[0])?;
        let b = require_object(&args[1])?;
        let merged = deep_merge_values(
            &serde_json::Value::Object(a.clone()),
            &serde_json::Value::Object(b.clone()),
        );
        Ok(NonFunctionValue::new(merged))
    }
}

// ── from_entries ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FromEntries;
impl Function for FromEntries {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let entries = require_array(arg)?;
        let mut result = serde_json::Map::new();
        for entry in entries {
            let pair = entry.as_array().ok_or(Error::TypeMismatch {
                expected: "array pair [key, value]",
                got: "non-array",
            })?;
            if pair.len() < 2 {
                return Err(Error::TypeMismatch {
                    expected: "pair with at least 2 elements",
                    got: "short pair",
                });
            }
            let key = pair[0].as_str().ok_or(Error::TypeMismatch {
                expected: "string key",
                got: "non-string",
            })?;
            result.insert(key.to_string(), pair[1].clone());
        }
        Ok(obj_value(result))
    }
}

// ── get ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Get;
impl Function for Get {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg) = require_two_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        match obj.get(&key) {
            Some(v) => Ok(NonFunctionValue::new(v.clone())),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
        }
    }
}

// ── get_or ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct GetOr;
impl Function for GetOr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg, default_arg) = require_three_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        match obj.get(&key) {
            Some(serde_json::Value::Null) | None => Ok(default_arg.clone()),
            Some(v) => Ok(NonFunctionValue::new(v.clone())),
        }
    }
}

// ── set ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Set;
impl Function for Set {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg, value_arg) = require_three_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        let mut result = obj.clone();
        result.insert(key, value_arg.inner().clone());
        Ok(obj_value(result))
    }
}

// ── update ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Update;
impl Function for Update {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg, fn_arg) = require_three_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        let fn_val = require_function_value(fn_arg)?;
        let current = obj.get(&key).cloned().unwrap_or(serde_json::Value::Null);
        let vars = bind_param(&fn_val, &current);
        let new_value = ctx.eval_expr(&fn_val.body, &vars)?;
        let mut result = obj.clone();
        result.insert(key, new_value.into_inner());
        Ok(obj_value(result))
    }
}

// ── has_key ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct HasKey;
impl Function for HasKey {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg) = require_two_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        Ok(b(obj.contains_key(&key)))
    }
}

// ── map_values ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct MapValues;
impl Function for MapValues {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let vars = bind_param(&fn_val, v);
            let new_value = ctx.eval_expr(&fn_val.body, &vars)?;
            result.insert(k.clone(), new_value.into_inner());
        }
        Ok(obj_value(result))
    }
}

// ── map_keys ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct MapKeys;
impl Function for MapKeys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let key_value = serde_json::Value::String(k.clone());
            let vars = bind_param(&fn_val, &key_value);
            let new_key = ctx.eval_expr(&fn_val.body, &vars)?;
            let new_key_str = new_key.as_str().ok_or(Error::TypeMismatch {
                expected: "string key from function",
                got: "non-string",
            })?;
            result.insert(new_key_str.to_string(), v.clone());
        }
        Ok(obj_value(result))
    }
}

// ── filter_keys ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FilterKeys;
impl Function for FilterKeys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let key_value = serde_json::Value::String(k.clone());
            let vars = bind_param(&fn_val, &key_value);
            let keep = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&keep) {
                result.insert(k.clone(), v.clone());
            }
        }
        Ok(obj_value(result))
    }
}

// ── filter_values ─────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FilterValues;
impl Function for FilterValues {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let vars = bind_param(&fn_val, v);
            let keep = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&keep) {
                result.insert(k.clone(), v.clone());
            }
        }
        Ok(obj_value(result))
    }
}

// ── invert ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Invert;
impl Function for Invert {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let new_key = value_to_string(v);
            result.insert(new_key, serde_json::Value::String(k.clone()));
        }
        Ok(obj_value(result))
    }
}

// ── path_or ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct PathOr;
impl Function for PathOr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (value_arg, path_arg, default_arg) = require_three_args(args)?;
        let path = require_string(path_arg)?;
        let mut current = value_arg.inner().clone();
        for segment in path.split('.') {
            if segment.is_empty() {
                continue;
            }
            current = match &current {
                serde_json::Value::Object(obj) => {
                    obj.get(segment).cloned().unwrap_or(serde_json::Value::Null)
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(idx) = segment.parse::<usize>() {
                        arr.get(idx).cloned().unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                _ => serde_json::Value::Null,
            };
            if current.is_null() {
                break;
            }
        }
        if current.is_null() {
            Ok(default_arg.clone())
        } else {
            Ok(NonFunctionValue::new(current))
        }
    }
}

// ── set_path ──────────────────────────────────────────────────────

fn set_path_recursive(
    value: serde_json::Value,
    segments: &[&str],
    new_value: serde_json::Value,
) -> serde_json::Value {
    if segments.is_empty() {
        return new_value;
    }
    let segment = segments[0];
    let rest = &segments[1..];
    let mut map = match value {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    let current = map.get(segment).cloned().unwrap_or(serde_json::Value::Null);
    let updated = set_path_recursive(current, rest, new_value);
    map.insert(segment.to_string(), updated);
    serde_json::Value::Object(map)
}

#[derive(Clone, Copy)]
pub struct SetPath;
impl Function for SetPath {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, path_arg, value_arg) = require_three_args(args)?;
        let path = require_string(path_arg)?;
        let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let result = set_path_recursive(
            obj_arg.inner().clone(),
            &segments,
            value_arg.inner().clone(),
        );
        Ok(NonFunctionValue::new(result))
    }
}

// ── update_path ───────────────────────────────────────────────────

fn update_path_recursive(
    value: serde_json::Value,
    segments: &[&str],
    fn_val: &FunctionValue,
    ctx: &mut Context,
) -> Result<serde_json::Value, Error> {
    if segments.is_empty() {
        let vars = bind_param(fn_val, &value);
        let result = ctx.eval_expr(&fn_val.body, &vars)?;
        return Ok(result.into_inner());
    }
    let segment = segments[0];
    let rest = &segments[1..];
    let mut map = match value {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    let current = map.get(segment).cloned().unwrap_or(serde_json::Value::Null);
    let updated = update_path_recursive(current, rest, fn_val, ctx)?;
    map.insert(segment.to_string(), updated);
    Ok(serde_json::Value::Object(map))
}

#[derive(Clone, Copy)]
pub struct UpdatePath;
impl Function for UpdatePath {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, path_arg, fn_arg) = require_three_args(args)?;
        let path = require_string(path_arg)?;
        let fn_val = require_function_value(fn_arg)?;
        let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let result = update_path_recursive(obj_arg.inner().clone(), &segments, &fn_val, ctx)?;
        Ok(NonFunctionValue::new(result))
    }
}

// ── size ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Size;
impl Function for Size {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let size = if arg.is_null() {
            0.0
        } else if let Some(arr) = arg.as_array() {
            arr.len() as f64
        } else if let Some(obj) = arg.as_object() {
            obj.len() as f64
        } else if let Some(s) = arg.as_str() {
            s.chars().count() as f64
        } else {
            return Err(Error::TypeMismatch {
                expected: "array, object, string, or null",
                got: "non-sized type",
            });
        };
        Ok(num(size))
    }
}

// ── clone_deep ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct CloneDeep;
impl Function for CloneDeep {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        Ok(NonFunctionValue::new(arg.inner().clone()))
    }
}
