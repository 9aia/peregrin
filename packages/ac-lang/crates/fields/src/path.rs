use crate::types::{DEFAULT_NAMESPACE, Namespaces};
use serde_json::Value as JsonValue;

/// Resolves a single path segment on a `JsonValue`, supporting both
/// object keys and array indices (numeric strings).
pub(crate) fn resolve_segment<'a>(value: &'a JsonValue, part: &str) -> Option<&'a JsonValue> {
    match value {
        JsonValue::Object(obj) => obj.get(part),
        JsonValue::Array(arr) => part.parse::<usize>().ok().and_then(|i| arr.get(i)),
        _ => None}
}

/// Resolves a path like `doc.name` or `env.HOME` or `name` (shorthand).
///
/// The path separator is provided by the dialect (e.g. `.` for the canonical
/// AC dialect).
///
/// Shorthand: if the first segment is not a known namespace, try the default
/// namespace (`DEFAULT_NAMESPACE`, conventionally `"doc"`).
pub(crate) fn resolve_path(
    expr: &str,
    namespaces: &Namespaces,
    path_separator: char,
) -> Option<JsonValue> {
    let parts: Vec<&str> = expr.split(path_separator).collect();
    if parts.is_empty() {
        return None;
    }

    let root = parts[0];
    let mut current: Option<&JsonValue> = namespaces.get(root);

    // Shorthand: if root is not a namespace, try the default namespace.
    if current.is_none() && !namespaces.contains_key(root) {
        current = namespaces
            .get(DEFAULT_NAMESPACE)
            .and_then(|d| resolve_segment(d, root));
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