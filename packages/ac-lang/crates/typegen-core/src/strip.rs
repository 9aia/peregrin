use serde_json::Value as JsonValue;

/// Recursively remove the given keys from a JSON value, in place.
///
/// Walks objects and arrays, removing any key in `keys` from object maps.
/// Non-object values are left unchanged. This avoids the deep cloning
/// that the returning variant [`strip_keys`] performs.
pub fn strip_keys_in_place(value: &mut JsonValue, keys: &[&str]) {
    match value {
        JsonValue::Object(map) => {
            map.retain(|key, _| !keys.contains(&key.as_str()));
            for (_, val) in map.iter_mut() {
                strip_keys_in_place(val, keys);
            }
        }
        JsonValue::Array(arr) => {
            for item in arr.iter_mut() {
                strip_keys_in_place(item, keys);
            }
        }
        _ => {}
    }
}

/// Recursively remove the given keys from a JSON value.
///
/// Walks objects and arrays, removing any key in `keys` from object maps.
/// Non-object values are returned unchanged.
///
/// Prefer [`strip_keys_in_place`] when you own the value — it avoids
/// a full deep clone.
pub fn strip_keys(value: &JsonValue, keys: &[&str]) -> JsonValue {
    let mut cloned = value.clone();
    strip_keys_in_place(&mut cloned, keys);
    cloned
}