use crate::types::Namespaces;
use serde_json::Value as JsonValue;

/// Resolves a single path segment on a `JsonValue`, supporting both
/// object keys and array indices (numeric strings).
pub(crate) fn resolve_segment<'a>(value: &'a JsonValue, part: &str) -> Option<&'a JsonValue> {
    match value {
        JsonValue::Object(obj) => obj.get(part),
        JsonValue::Array(arr) => part.parse::<usize>().ok().and_then(|i| arr.get(i)),
        _ => None,
    }
}

/// Resolves a dotted path like `doc.name` or `env.HOME` or `name` (shorthand).
///
/// Shorthand: if the first segment is not a known namespace, try `doc`.
pub(crate) fn resolve_path(expr: &str, namespaces: &Namespaces) -> Option<JsonValue> {
    let parts: Vec<&str> = expr.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let root = parts[0];
    let mut current: Option<&JsonValue> = namespaces.get(root);

    // Shorthand: if root is not a namespace, try doc.
    if current.is_none() && !namespaces.contains_key(root) {
        current = namespaces.get("doc").and_then(|d| resolve_segment(d, root));
        for part in parts.iter().skip(1) {
            current = current.and_then(|v| resolve_segment(v, part));
        }
        return current.cloned();
    }

    for part in parts.iter().skip(1) {
        current = current.and_then(|v| resolve_segment(v, part));
    }

    current.cloned()
}
