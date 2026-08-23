use serde_json::Value as JsonValue;

/// Produce a canonical JSON value with sorted object keys.
///
/// Canonicalization ensures that two semantically identical values
/// produce the same hash regardless of key order. Arrays preserve
/// their order (order is semantically significant for arrays).
pub fn canonicalize_json(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), canonicalize_json(&obj[key]));
            }
            JsonValue::Object(sorted)
        }
        JsonValue::Array(arr) => JsonValue::Array(arr.iter().map(canonicalize_json).collect()),
        other => other.clone(),
    }
}

/// Serialize a JSON value to a canonical string (sorted keys, no
/// whitespace).
pub fn canonical_string(value: &JsonValue) -> String {
    let canonical = canonicalize_json(value);
    serde_json::to_string(&canonical).unwrap_or_default()
}
