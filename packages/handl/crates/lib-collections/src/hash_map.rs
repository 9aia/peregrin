//! `collections.hash_map` — key/value dictionary backed by a hash table.
//!
//! Currently implemented as a wrapper around `serde_json::Value::Object`.

use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, NonFunctionValue};
use serde_json::Map;

use super::common::{arr, b, num, obj, require_object, require_one_object};

fn require_key(args: &[NonFunctionValue], idx: usize) -> Result<&str, Error> {
    let arg = args.get(idx).ok_or(Error::ArgumentCount {
        expected: idx + 1,
        got: args.len(),
    })?;
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

/// `collections.hash_map.new() → object`
#[derive(Clone, Copy)]
pub struct New;

impl Function<NonFunctionValue> for New {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(obj(Map::new()))
    }
}

/// `collections.hash_map.from(object: object) → object`
#[derive(Clone, Copy)]
pub struct From;

impl Function<NonFunctionValue> for From {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_object(args)?;
        Ok(obj(input.clone()))
    }
}

/// `collections.hash_map.to_object(map: hash_map) → object`
#[derive(Clone, Copy)]
pub struct ToObject;

impl Function<NonFunctionValue> for ToObject {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_object(args)?;
        Ok(obj(input.clone()))
    }
}

/// `collections.hash_map.len(map: hash_map) → number`
#[derive(Clone, Copy)]
pub struct Len;

impl Function<NonFunctionValue> for Len {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_object(args)?;
        Ok(num(input.len() as f64))
    }
}

/// `collections.hash_map.is_empty(map: hash_map) → bool`
#[derive(Clone, Copy)]
pub struct IsEmpty;

impl Function<NonFunctionValue> for IsEmpty {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_object(args)?;
        Ok(b(input.is_empty()))
    }
}

/// `collections.hash_map.get(map: hash_map, key: string, default?: value) → value`
#[derive(Clone, Copy)]
pub struct Get;

impl Function<NonFunctionValue> for Get {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_object(args, 0)?;
        let key = require_key(args, 1)?;

        match input.get(key) {
            Some(value) => Ok(NonFunctionValue::new(value.clone())),
            None => match args.get(2) {
                Some(default) => Ok(NonFunctionValue::new(default.inner().clone())),
                None => Ok(NonFunctionValue::new(serde_json::Value::Null)),
            },
        }
    }
}

/// `collections.hash_map.set(map: hash_map, key: string, value: value) → hash_map`
#[derive(Clone, Copy)]
pub struct Set;

impl Function<NonFunctionValue> for Set {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_object(args, 0)?;
        let key = require_key(args, 1)?;
        let value = args.get(2).ok_or(Error::ArgumentCount {
            expected: 3,
            got: args.len(),
        })?;

        let mut map = input.clone();
        map.insert(key.to_string(), value.inner().clone());
        Ok(obj(map))
    }
}

/// `collections.hash_map.remove(map: hash_map, key: string) → hash_map`
#[derive(Clone, Copy)]
pub struct Remove;

impl Function<NonFunctionValue> for Remove {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_object(args, 0)?;
        let key = require_key(args, 1)?;
        let mut map = input.clone();
        map.remove(key);
        Ok(obj(map))
    }
}

/// `collections.hash_map.has(map: hash_map, key: string) → bool`
#[derive(Clone, Copy)]
pub struct Has;

impl Function<NonFunctionValue> for Has {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_object(args, 0)?;
        let key = require_key(args, 1)?;
        Ok(b(input.contains_key(key)))
    }
}

/// `collections.hash_map.keys(map: hash_map) → array<string>`
#[derive(Clone, Copy)]
pub struct Keys;

impl Function<NonFunctionValue> for Keys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_object(args)?;
        let keys: Vec<serde_json::Value> = input
            .keys()
            .map(|k| serde_json::Value::String(k.clone()))
            .collect();
        Ok(arr(&keys))
    }
}

/// `collections.hash_map.values(map: hash_map) → array`
#[derive(Clone, Copy)]
pub struct Values;

impl Function<NonFunctionValue> for Values {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_object(args)?;
        let values: Vec<serde_json::Value> = input.values().cloned().collect();
        Ok(arr(&values))
    }
}
