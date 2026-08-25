use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, NonFunctionValue};
use serde_json::Value as JsonValue;

fn require_arg_count(args: &[NonFunctionValue], expected: usize) -> Result<(), Error> {
    if args.len() != expected {
        return Err(Error::ArgumentCount {
            expected,
            got: args.len(),
        });
    }
    Ok(())
}

fn as_kind(value: &NonFunctionValue) -> Result<&str, Error> {
    value
        .as_str()
        .ok_or_else(|| Error::PrimitiveError("mood kind must be a string".to_string()))
}

fn bool_value(b: bool) -> NonFunctionValue {
    NonFunctionValue::new(b)
}

fn null_value() -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Null)
}

fn type_name(value: &NonFunctionValue) -> &'static str {
    if value.is_null() {
        "null"
    } else if value.is_bool() {
        "bool"
    } else if value.is_number() {
        "number"
    } else if value.is_string() {
        "string"
    } else if value.is_array() {
        "array"
    } else if value.is_object() {
        "object"
    } else {
        "unknown"
    }
}

fn matches(value: &NonFunctionValue, kind: &str) -> bool {
    if let Some(rest) = kind.strip_prefix("type.") {
        match rest {
            "null" => value.is_null(),
            "bool" => value.is_bool(),
            "number" => value.is_number(),
            "string" => value.is_string(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => false,
        }
    } else if let Some(rest) = kind.strip_prefix("field.") {
        value.as_object().is_some_and(|o| o.get(rest).is_some())
    } else {
        value.as_object().is_some_and(|o| o.get(kind).is_some())
    }
}

fn project(value: &NonFunctionValue, kind: &str) -> Option<NonFunctionValue> {
    if kind.starts_with("type.") {
        if matches(value, kind) {
            Some(NonFunctionValue::new(value.inner().clone()))
        } else {
            None
        }
    } else if let Some(rest) = kind.strip_prefix("field.") {
        value
            .as_object()
            .and_then(|o| o.get(rest))
            .map(|v| NonFunctionValue::new(v.clone()))
    } else {
        value
            .as_object()
            .and_then(|o| o.get(kind))
            .map(|v| NonFunctionValue::new(v.clone()))
    }
}

fn complement(value: &NonFunctionValue, kind: &str) -> Option<NonFunctionValue> {
    if matches(value, kind) {
        return None;
    }

    if kind.starts_with("type.") || kind.starts_with("field.") {
        return Some(NonFunctionValue::new(value.inner().clone()));
    }

    // Variant tag: the complement is another key in the same object,
    // or the value itself if it is not a tagged object.
    if let Some(obj) = value.as_object() {
        obj.iter()
            .find(|(k, _)| k.as_str() != kind)
            .map(|(_, v)| NonFunctionValue::new(v.clone()))
    } else {
        Some(NonFunctionValue::new(value.inner().clone()))
    }
}

fn ok_result(value: NonFunctionValue) -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Object(serde_json::Map::from_iter([(
        "ok".to_string(),
        value.into_inner(),
    )])))
}

fn err_result(message: &str) -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Object(serde_json::Map::from_iter([(
        "err".to_string(),
        JsonValue::String(message.to_string()),
    )])))
}

#[derive(Clone, Copy)]
pub struct MoodInd;
impl Function<NonFunctionValue> for MoodInd {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_arg_count(args, 2)?;
        let value = &args[0];
        let kind = as_kind(&args[1])?;
        match project(value, kind) {
            Some(v) => Ok(v),
            None => Err(Error::PrimitiveError(format!(
                "mood.ind: expected {kind}, got {}",
                type_name(value)
            ))),
        }
    }
}

#[derive(Clone, Copy)]
pub struct MoodInt;
impl Function<NonFunctionValue> for MoodInt {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_arg_count(args, 2)?;
        let value = &args[0];
        let kind = as_kind(&args[1])?;
        Ok(bool_value(matches(value, kind)))
    }
}

#[derive(Clone, Copy)]
pub struct MoodNorm;
impl Function<NonFunctionValue> for MoodNorm {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_arg_count(args, 2)?;
        let value = &args[0];
        let kind = as_kind(&args[1])?;
        match project(value, kind) {
            Some(v) => Ok(ok_result(v)),
            None => Ok(err_result(&format!("mood.norm: expected {kind}"))),
        }
    }
}

#[derive(Clone, Copy)]
pub struct MoodPot;
impl Function<NonFunctionValue> for MoodPot {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_arg_count(args, 2)?;
        let value = &args[0];
        let kind = as_kind(&args[1])?;
        Ok(project(value, kind).unwrap_or_else(null_value))
    }
}

#[derive(Clone, Copy)]
pub struct MoodNeg;
impl Function<NonFunctionValue> for MoodNeg {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_arg_count(args, 2)?;
        let value = &args[0];
        let kind = as_kind(&args[1])?;
        Ok(complement(value, kind).unwrap_or_else(null_value))
    }
}

#[derive(Clone, Copy)]
pub struct MoodCnd;
impl Function<NonFunctionValue> for MoodCnd {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_arg_count(args, 3)?;
        let value = &args[0];
        let kind = as_kind(&args[1])?;
        let default = &args[2];
        Ok(project(value, kind).unwrap_or_else(|| NonFunctionValue::new(default.inner().clone())))
    }
}

#[derive(Clone, Copy)]
pub struct MoodModule;
impl Module for MoodModule {
    fn name(&self) -> &str {
        "mood"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "ind" => Some(&MoodInd),
            "int" => Some(&MoodInt),
            "norm" => Some(&MoodNorm),
            "pot" => Some(&MoodPot),
            "neg" => Some(&MoodNeg),
            "cnd" => Some(&MoodCnd),
            _ => None,
        }
    }
}
