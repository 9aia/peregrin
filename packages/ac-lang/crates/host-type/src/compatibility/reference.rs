//! Rust reference implementation of compatibility for A/B testing.
//!
//! Mirrors `ac_typecheck_core::is_compatible` but operates on JSON-encoded
//! types rather than the `Type` enum.

use serde_json::Value as JsonValue;

/// Check if `actual` is compatible with `expected`, operating on JSON-encoded types.
///
/// This mirrors `ac_typecheck_core::is_compatible` but works on
/// [`JsonValue`] instead of [`Type`].
pub fn is_compatible_json(actual: &JsonValue, expected: &JsonValue) -> bool {
    let actual_kind = kind_of(actual);
    let expected_kind = kind_of(expected);

    if actual_kind == "unknown"
        || expected_kind == "unknown"
        || actual_kind == "never"
        || expected_kind == "never"
    {
        return true;
    }

    match (actual_kind, expected_kind) {
        ("var", "var") => name_of(actual) == name_of(expected),
        ("trait", "trait") => name_of(actual) == name_of(expected),
        ("app", "app") => {
            let a_ctor = field(actual, "constructor").unwrap_or(&JsonValue::Null);
            let e_ctor = field(expected, "constructor").unwrap_or(&JsonValue::Null);
            let a_args = field(actual, "args").and_then(|v| v.as_array());
            let e_args = field(expected, "args").and_then(|v| v.as_array());
            match (a_args, e_args) {
                (Some(a), Some(e)) => {
                    is_compatible_json(a_ctor, e_ctor)
                        && a.len() == e.len()
                        && a.iter().zip(e).all(|(a, e)| is_compatible_json(a, e))
                }
                _ => false}
        }
        ("string", "string") | ("number", "number") | ("boolean", "boolean") | ("null", "null") => {
            true
        }
        ("array", "array") => {
            let a_item = field(actual, "item").unwrap_or(&JsonValue::Null);
            let e_item = field(expected, "item").unwrap_or(&JsonValue::Null);
            is_compatible_json(a_item, e_item)
        }
        ("object", "object") => {
            let e_props = match field(expected, "properties").and_then(|v| v.as_object()) {
                Some(p) => p,
                None => return false};
            let a_props = match field(actual, "properties").and_then(|v| v.as_object()) {
                Some(p) => p,
                None => return false};
            e_props.iter().all(|(key, e_ty)| {
                a_props
                    .get(key)
                    .map(|a_ty| is_compatible_json(a_ty, e_ty))
                    .unwrap_or(false)
            })
        }
        ("function", "function") => {
            let a_params = field(actual, "params").and_then(|v| v.as_array());
            let e_params = field(expected, "params").and_then(|v| v.as_array());
            match (a_params, e_params) {
                (Some(a), Some(e)) => {
                    if a.len() != e.len() {
                        return false;
                    }
                    let a_ret = field(actual, "return").unwrap_or(&JsonValue::Null);
                    let e_ret = field(expected, "return").unwrap_or(&JsonValue::Null);
                    a.iter().zip(e).all(|(a_pair, e_pair)| {
                        let a_ty = pair_type(a_pair);
                        let e_ty = pair_type(e_pair);
                        is_compatible_json(e_ty, a_ty)
                    }) && is_compatible_json(a_ret, e_ret)
                }
                _ => false}
        }
        _ => false}
}

fn kind_of(v: &JsonValue) -> &str {
    v.as_object()
        .and_then(|o| o.get("kind"))
        .and_then(|k| k.as_str())
        .unwrap_or("")
}

fn name_of(v: &JsonValue) -> &str {
    v.as_object()
        .and_then(|o| o.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
}

fn field<'a>(v: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    v.as_object().and_then(|o| o.get(key))
}

fn pair_type(pair: &JsonValue) -> &JsonValue {
    pair.as_array()
        .and_then(|a| a.get(1))
        .unwrap_or(&JsonValue::Null)
}
