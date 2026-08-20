use serde_json::Value as JsonValue;

use crate::AC_ONLY_KEYS;

/// Remove AC-specific keys from a field schema fragment.
///
/// The input is a raw schema value that may contain AC-specific keys
/// (`expr`, `arg`, `merge`, `inheritable`, `mutable`) alongside schema
/// vocabulary (`type`, `required`, `properties`, etc.).
///
/// The output is a clean schema fragment with only the schema vocabulary
/// remaining. Non-object values are returned unchanged.
pub fn strip_ac_keys(raw: &JsonValue) -> JsonValue {
    match raw {
        JsonValue::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, value) in map {
                if !AC_ONLY_KEYS.contains(&key.as_str()) {
                    cleaned.insert(key.clone(), strip_recursive(value));
                }
            }
            JsonValue::Object(cleaned)
        }
        other => other.clone(),
    }
}

/// Recursively strip AC keys from nested objects within a schema.
fn strip_recursive(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, val) in map {
                if !AC_ONLY_KEYS.contains(&key.as_str()) {
                    cleaned.insert(key.clone(), strip_recursive(val));
                }
            }
            JsonValue::Object(cleaned)
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(strip_recursive).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_all_ac_keys() {
        let input = json!({
            "type": "string",
            "expr": "concat(arg, \"!\")",
            "arg": true,
            "merge": true,
            "inheritable": true,
            "mutable": false,
            "pattern": "^[a-z]+$"
        });
        let output = strip_ac_keys(&input);
        assert_eq!(output, json!({
            "type": "string",
            "pattern": "^[a-z]+$"
        }));
    }

    #[test]
    fn preserves_nested_schema_keys() {
        let input = json!({
            "type": "object",
            "properties": {
                "host": { "type": "string", "expr": "something()" },
                "port": { "type": "number" }
            },
            "required": ["host"]
        });
        let output = strip_ac_keys(&input);
        assert_eq!(output, json!({
            "type": "object",
            "properties": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            },
            "required": ["host"]
        }));
    }

    #[test]
    fn strips_ac_keys_in_nested_arrays() {
        let input = json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "expr": "compute()" }
                }
            }
        });
        let output = strip_ac_keys(&input);
        assert_eq!(output, json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }
        }));
    }

    #[test]
    fn non_object_returned_unchanged() {
        let input = json!("string");
        let output = strip_ac_keys(&input);
        assert_eq!(output, input);
    }

    #[test]
    fn empty_object_stays_empty() {
        let input = json!({});
        let output = strip_ac_keys(&input);
        assert_eq!(output, json!({}));
    }

    #[test]
    fn object_with_only_ac_keys_becomes_empty() {
        let input = json!({ "expr": "foo()", "arg": true, "merge": true });
        let output = strip_ac_keys(&input);
        assert_eq!(output, json!({}));
    }
}
