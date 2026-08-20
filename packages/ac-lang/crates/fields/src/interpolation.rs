use crate::expression::eval_expr_inner;
use crate::types::{FunctionResolver, Namespaces, DEFAULT_NAMESPACE};
use serde_json::Value as JsonValue;

/// Recursively interpolates `${...}` patterns in string values.
///
/// Supports:
/// - Namespace paths: `${doc.name}`, `${env.HOME}`, `${doc.author.email}`
/// - Shorthand paths: `${name}` (equivalent to `${doc.name}`)
/// - Function calls: `${now()}`, `${add(1, 2)}`, `${concat(doc.name, "!")}`
/// - Nested calls: `${add(mul(2, 3), 4)}`
///
/// Function calls are resolved via `resolver`. If no resolver is needed,
/// use `NoFunctions`.
pub fn interpolate(
    value: &JsonValue,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
) -> JsonValue {
    match value {
        JsonValue::String(s) if s.contains("${") => {
            resolve_interpolation(s, namespaces, resolver)
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(|v| interpolate(v, namespaces, resolver)).collect())
        }
        JsonValue::Object(obj) => JsonValue::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), interpolate(v, namespaces, resolver)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Recursively interpolates `${...}` patterns with scope inheritance.
///
/// Like [`interpolate`], but as the recursion descends into nested objects,
/// fields declared as `inheritable: true` are merged into the default namespace.
/// Child objects inherit the parent's scope and can override inheritable fields
/// locally — the override applies to that subtree only.
///
/// If `inheritable` is empty, this is equivalent to [`interpolate`].
pub fn interpolate_scoped(
    value: &JsonValue,
    namespaces: &Namespaces,
    inheritable: &[String],
    resolver: &dyn FunctionResolver,
) -> JsonValue {
    if inheritable.is_empty() {
        return interpolate(value, namespaces, resolver);
    }
    let inheritable_set: std::collections::HashSet<String> =
        inheritable.iter().cloned().collect();
    let empty_scope = serde_json::Map::new();
    interpolate_with_scope(value, namespaces, &inheritable_set, &empty_scope, resolver)
}

/// Internal: scope-aware interpolation. Carries a `scope` (inheritable field
/// values from parent objects) and merges it into the default namespace at each
/// object level.
fn interpolate_with_scope(
    value: &JsonValue,
    namespaces: &Namespaces,
    inheritable: &std::collections::HashSet<String>,
    scope: &serde_json::Map<String, JsonValue>,
    resolver: &dyn FunctionResolver,
) -> JsonValue {
    match value {
        JsonValue::Object(obj) => {
            // Build current scope: parent scope + local inheritable overrides
            let mut current_scope = scope.clone();
            for (key, val) in obj {
                if inheritable.contains(key) {
                    current_scope.insert(key.clone(), val.clone());
                }
            }

            // Build merged namespace for children: default namespace + scope
            let mut child_ns = namespaces.clone();
            let doc_obj = child_ns
                .entry(DEFAULT_NAMESPACE.to_string())
                .or_insert(JsonValue::Object(serde_json::Map::new()));
            if let Some(doc_map) = doc_obj.as_object_mut() {
                for (key, val) in &current_scope {
                    doc_map.insert(key.clone(), val.clone());
                }
            }

            // Recurse with updated scope and namespace
            JsonValue::Object(
                obj.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            interpolate_with_scope(v, &child_ns, inheritable, &current_scope, resolver),
                        )
                    })
                    .collect(),
            )
        }
        JsonValue::String(s) if s.contains("${") => {
            resolve_interpolation(s, namespaces, resolver)
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(
                arr.iter()
                    .map(|v| interpolate_with_scope(v, namespaces, inheritable, scope, resolver))
                    .collect(),
            )
        }
        _ => value.clone(),
    }
}

/// Checks if `s` is exactly a single `${...}` expression (nothing before or after).
/// Returns the inner expression text if so.
fn extract_single_expr(s: &str) -> Option<&str> {
    let trimmed = s.strip_prefix("${")?;
    let end = trimmed.rfind('}')?;
    if end == trimmed.len() - 1 {
        Some(&trimmed[..end])
    } else {
        None
    }
}

/// Replaces all `${...}` patterns in a string, returning a `JsonValue`.
/// If the string is a single `${...}`, the raw value is returned (preserving type).
/// Otherwise, all `${...}` patterns are stringified and concatenated.
fn resolve_interpolation(
    s: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
) -> JsonValue {
    // Single expression → return raw value
    if let Some(inner) = extract_single_expr(s)
        && let Some(val) = eval_expr_inner(inner, namespaces, resolver)
    {
        return val;
    }

    // Multiple expressions → stringify and concatenate
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find("${") {
        result.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        if let Some(end) = after_start.find('}') {
            let expr = &after_start[..end];
            let val = eval_expr_inner(expr, namespaces, resolver);
            match val {
                Some(JsonValue::String(t)) => result.push_str(&t),
                Some(JsonValue::Number(n)) => result.push_str(&n.to_string()),
                Some(JsonValue::Bool(b)) => result.push_str(&b.to_string()),
                Some(JsonValue::Null) => result.push_str("null"),
                Some(v) => result.push_str(&v.to_string()),
                None => result.push_str(&format!("${{{expr}}}")),
            }
            remaining = &after_start[end + 1..];
        } else {
            result.push_str("${");
            remaining = after_start;
        }
    }
    result.push_str(remaining);
    JsonValue::String(result)
}
