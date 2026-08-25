//! The `unicode` handl runtime module.
//!
//! Exposes Unicode helpers callable from handl expressions:
//!
//! - `unicode.is_valid_identifier(string)` — `bool`
//! - `unicode.is_xid_start(string)` — `bool`
//! - `unicode.is_xid_continue(string)` — `bool`
//! - `unicode.code_points(string)` — array of numeric code points
//! - `unicode.code_point_at(string, index)` — number or `null`
//! - `unicode.decode(string)` — array of `[byte_offset, code_point]` pairs

use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, NonFunctionValue};
use serde_json::Value as JsonValue;

use super::decode::Utf8Iterator;
use super::ident::{is_identifier, is_identifier_continue, is_identifier_start};

/// The `unicode` module — Unicode primitives for handl expressions.
#[derive(Clone, Copy)]
pub struct UnicodeModule;

impl Module for UnicodeModule {
    fn name(&self) -> &str {
        "unicode"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "is_valid_identifier" => Some(&IsValidIdentifier),
            "is_xid_start" => Some(&IsXidStart),
            "is_xid_continue" => Some(&IsXidContinue),
            "code_points" => Some(&CodePoints),
            "code_point_at" => Some(&CodePointAt),
            "decode" => Some(&Decode),
            _ => None,
        }
    }
}

fn require_one_string(args: &[NonFunctionValue]) -> Result<String, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: args.len(),
    })?;
    arg.as_str()
        .map(|s| s.to_string())
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn require_string_and_index(args: &[NonFunctionValue]) -> Result<(String, usize), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let s = args[0]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
    let n = args[1].as_number().ok_or(Error::TypeMismatch {
        expected: "number",
        got: "non-number",
    })?;
    if n < 0.0 || n.fract() != 0.0 {
        return Err(Error::TypeMismatch {
            expected: "non-negative integer",
            got: "invalid index",
        });
    }
    Ok((s, n as usize))
}

fn bool_(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

fn null() -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Null)
}

#[derive(Clone, Copy)]
pub struct IsValidIdentifier;
impl Function<NonFunctionValue> for IsValidIdentifier {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let s = require_one_string(args)?;
        Ok(bool_(is_identifier(&s)))
    }
}

#[derive(Clone, Copy)]
pub struct IsXidStart;
impl Function<NonFunctionValue> for IsXidStart {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let s = require_one_string(args)?;
        let first = s.chars().next();
        Ok(bool_(first.map_or(false, is_identifier_start)))
    }
}

#[derive(Clone, Copy)]
pub struct IsXidContinue;
impl Function<NonFunctionValue> for IsXidContinue {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let s = require_one_string(args)?;
        let first = s.chars().next();
        Ok(bool_(first.map_or(false, is_identifier_continue)))
    }
}

#[derive(Clone, Copy)]
pub struct CodePoints;
impl Function<NonFunctionValue> for CodePoints {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let s = require_one_string(args)?;
        let points: Vec<JsonValue> = s
            .chars()
            .map(|ch| JsonValue::Number(serde_json::Number::from(ch as u32)))
            .collect();
        Ok(NonFunctionValue::new(JsonValue::Array(points)))
    }
}

#[derive(Clone, Copy)]
pub struct CodePointAt;
impl Function<NonFunctionValue> for CodePointAt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (s, index) = require_string_and_index(args)?;
        match s.chars().nth(index) {
            Some(ch) => Ok(num(ch as u32 as f64)),
            None => Ok(null()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Decode;
impl Function<NonFunctionValue> for Decode {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let s = require_one_string(args)?;
        let mut pairs = Vec::new();
        for item in Utf8Iterator::new(s.as_bytes()) {
            let (offset, ch) =
                item.map_err(|_| Error::PrimitiveError("invalid UTF-8 in string".to_string()))?;
            let pair = JsonValue::Array(vec![
                JsonValue::Number(serde_json::Number::from(offset)),
                JsonValue::Number(serde_json::Number::from(ch as u32)),
            ]);
            pairs.push(pair);
        }
        Ok(NonFunctionValue::new(JsonValue::Array(pairs)))
    }
}
