use handl_vm::{Context, Error, Pattern};
use handl_vm_values_json::NonFunctionValue;
use serde_json::Value as JsonValue;

/// Defines each name bound by `pattern` to the matching part of
/// `value`, then returns the original value.
pub fn define_pattern(
    ctx: &mut dyn Context<NonFunctionValue>,
    pattern: &Pattern,
    value: NonFunctionValue,
) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
    define_pattern_inner(ctx, pattern, value.inner())?;
    Ok(value)
}

fn define_pattern_inner(
    ctx: &mut dyn Context<NonFunctionValue>,
    pattern: &Pattern,
    value: &JsonValue,
) -> Result<(), Error<NonFunctionValue>> {
    match pattern {
        Pattern::Identifier(name) => {
            ctx.define(name, NonFunctionValue::new(value.clone()));
        }
        Pattern::Array(elements) => {
            let arr = value.as_array().ok_or(Error::TypeMismatch {
                expected: "array",
                got: "non-array",
            })?;
            for (i, element) in elements.iter().enumerate() {
                let child = arr.get(i).unwrap_or(&JsonValue::Null);
                define_pattern_inner(ctx, element, child)?;
            }
        }
        Pattern::Object(entries) => {
            let obj = value.as_object().ok_or(Error::TypeMismatch {
                expected: "object",
                got: "non-object",
            })?;
            for (key, child_pattern) in entries {
                let child = obj.get(key).unwrap_or(&JsonValue::Null);
                define_pattern_inner(ctx, child_pattern, child)?;
            }
        }
        Pattern::Rest(_name) => {
            // Rest patterns are not supported at runtime yet; bind nothing.
        }
    }
    Ok(())
}

/// Serializes a [`Pattern`] to a JSON value.
///
/// The JSON format is:
/// - `{"type":"identifier","name":"x"}`
/// - `{"type":"array","patterns":[...]}`
/// - `{"type":"object","entries":[["key", pattern], ...]}`
/// - `{"type":"rest","name":"rest"}`
pub fn pattern_to_json(pattern: &Pattern) -> JsonValue {
    match pattern {
        Pattern::Identifier(name) => serde_json::json!({
            "type": "identifier",
            "name": name,
        }),
        Pattern::Array(patterns) => serde_json::json!({
            "type": "array",
            "patterns": patterns.iter().map(pattern_to_json).collect::<Vec<_>>(),
        }),
        Pattern::Object(entries) => serde_json::json!({
            "type": "object",
            "entries": entries
                .iter()
                .map(|(k, p)| serde_json::json!([k, pattern_to_json(p)]))
                .collect::<Vec<_>>(),
        }),
        Pattern::Rest(name) => serde_json::json!({
            "type": "rest",
            "name": name,
        }),
    }
}

/// Deserializes a [`Pattern`] from JSON.
pub fn pattern_from_json(value: &JsonValue) -> Option<Pattern> {
    let obj = value.as_object()?;
    match obj.get("type").and_then(|v| v.as_str())? {
        "identifier" => {
            let name = obj.get("name").and_then(|v| v.as_str())?.to_string();
            Some(Pattern::Identifier(name))
        }
        "array" => {
            let patterns = obj
                .get("patterns")
                .and_then(|v| v.as_array())?
                .iter()
                .map(pattern_from_json)
                .collect::<Option<Vec<_>>>()?;
            Some(Pattern::Array(patterns))
        }
        "object" => {
            let entries = obj
                .get("entries")
                .and_then(|v| v.as_array())?
                .iter()
                .map(|entry| {
                    let arr = entry.as_array()?;
                    let key = arr.get(0).and_then(|v| v.as_str())?.to_string();
                    let pattern = pattern_from_json(arr.get(1)?)?;
                    Some((key, pattern))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Pattern::Object(entries))
        }
        "rest" => {
            let name = obj.get("name").and_then(|v| v.as_str())?.to_string();
            Some(Pattern::Rest(name))
        }
        _ => None,
    }
}
