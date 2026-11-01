//! Parse helpers — `readTextFile`/`parseYaml`/`parseJson` ports.
//!
//! YAML parses through `serde_yaml_ng::Value` (which allows non-string
//! keys) and converts to `serde_json::Value`, coercing map keys the way
//! the JS `yaml` package's plain-object output does — `emit: {0: …}`
//! lands as `"0"`.

use std::path::Path;

use peregrin_loader::LoadError;
use serde_json::{Map, Value};

/// TS `readTextFile` — `Failed to read file: <path>` with the io error
/// as the cause.
pub(crate) fn read_text_file(path: &Path) -> Result<String, LoadError> {
    std::fs::read_to_string(path)
        .map_err(|e| LoadError::caused(format!("Failed to read file: {}", path.display()), e))
}

/// TS `parseYaml` — the error's `Display` carries line/column like the
/// `yaml` package's `YAMLError`.
pub(crate) fn parse_yaml(text: &str) -> Result<Value, LoadError> {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).map_err(|e| LoadError::caused("Failed to parse YAML", e))?;
    Ok(yaml_to_json(&yaml))
}

/// TS `parseJson` — used for `*.event.json` payloads.
pub(crate) fn parse_json(text: &str) -> Result<Value, LoadError> {
    serde_json::from_str(text).map_err(|e| LoadError::caused("Failed to parse JSON", e))
}

/// JSON-map key coercion matching JS object-key semantics (`String(k)`).
fn key_string(value: &serde_yaml_ng::Value) -> String {
    use serde_yaml_ng::Value as Y;
    match value {
        Y::Null => "null".to_string(),
        Y::Bool(b) => b.to_string(),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                n.as_f64().map_or_else(
                    || n.to_string(),
                    |f| {
                        if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e21 {
                            format!("{}", f as i64)
                        } else {
                            n.to_string()
                        }
                    },
                )
            }
        }
        Y::String(s) => s.clone(),
        Y::Sequence(items) => items.iter().map(key_string).collect::<Vec<_>>().join(","),
        // Non-scalar keys coerce like JS `[object Object]`; tagged keys
        // unwrap to their inner value.
        Y::Mapping(_) => "[object Object]".to_string(),
        Y::Tagged(tagged) => key_string(&tagged.value),
    }
}

fn yaml_to_json(value: &serde_yaml_ng::Value) -> Value {
    use serde_yaml_ng::Value as Y;
    match value {
        Y::Null => Value::Null,
        Y::Bool(b) => Value::Bool(*b),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                Value::Number(u.into())
            } else {
                n.as_f64()
                    .and_then(serde_json::Number::from_f64)
                    .map_or(Value::Null, Value::Number)
            }
        }
        Y::String(s) => Value::String(s.clone()),
        Y::Sequence(items) => Value::Array(items.iter().map(yaml_to_json).collect()),
        Y::Mapping(map) => {
            let mut out = Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(key_string(k), yaml_to_json(v));
            }
            Value::Object(out)
        }
        Y::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}
