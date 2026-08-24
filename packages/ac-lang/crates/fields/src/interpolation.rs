use ac_runtime_values::NonFunctionValue;
use crate::expression::eval_expression;
use crate::types::{DEFAULT_NAMESPACE, Namespaces};
use ac_dialect::Dialect;
use ac_runtime::Host;
use serde_json::Value as JsonValue;

/// Recursively interpolates expressions in string values.
///
/// Supports (using the canonical AC dialect):
/// - Namespace paths: `${doc.name}`, `${env.HOME}`, `${doc.author.email}`
/// - Shorthand paths: `${name}` (equivalent to `${doc.name}`)
/// - Function calls: `${now()}`, `${add(1, 2)}`, `${concat(doc.name, "!")}`
/// - Nested calls: `${add(mul(2, 3), 4)}`
///
/// Function calls are resolved via `host`. If no functions are needed,
/// use `ac_runtime_example_1::DefaultHost`.
pub fn interpolate<H: Host<NonFunctionValue> + Clone + 'static>(
    value: &JsonValue,
    namespaces: &Namespaces,
    host: &H,
    dialect: &'static dyn Dialect,
) -> JsonValue {
    let prefix = dialect.syntax().interpolation_prefix();
    match value {
        JsonValue::String(s) if s.contains(prefix) => {
            resolve_interpolation(s, namespaces, host, dialect)
        }
        JsonValue::Array(arr) => JsonValue::Array(
            arr.iter()
                .map(|v| interpolate(v, namespaces, host, dialect))
                .collect(),
        ),
        JsonValue::Object(obj) => JsonValue::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), interpolate(v, namespaces, host, dialect)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Recursively interpolates expressions in string values with scope inheritance.
///
/// Like [`interpolate`], but as the recursion descends into nested objects,
/// fields declared as `inheritable: true` are merged into the default namespace.
/// Child objects inherit the parent's scope and can override inheritable fields
/// locally — the override applies to that subtree only.
///
/// If `inheritable` is empty, this is equivalent to [`interpolate`].
pub fn interpolate_scoped<H: Host<NonFunctionValue> + Clone + 'static>(
    value: &JsonValue,
    namespaces: &Namespaces,
    inheritable: &[String],
    host: &H,
    dialect: &'static dyn Dialect,
) -> JsonValue {
    if inheritable.is_empty() {
        return interpolate(value, namespaces, host, dialect);
    }
    let inheritable_set: std::collections::HashSet<String> = inheritable.iter().cloned().collect();
    let empty_scope = serde_json::Map::new();
    interpolate_with_scope(
        value,
        namespaces,
        &inheritable_set,
        &empty_scope,
        host,
        dialect,
    )
}

/// Internal: scope-aware interpolation. Carries a `scope` (inheritable field
/// values from parent objects) and merges it into the default namespace at each
/// object level.
fn interpolate_with_scope<H: Host<NonFunctionValue> + Clone + 'static>(
    value: &JsonValue,
    namespaces: &Namespaces,
    inheritable: &std::collections::HashSet<String>,
    scope: &serde_json::Map<String, JsonValue>,
    host: &H,
    dialect: &'static dyn Dialect,
) -> JsonValue {
    let prefix = dialect.syntax().interpolation_prefix();
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
                            interpolate_with_scope(
                                v,
                                &child_ns,
                                inheritable,
                                &current_scope,
                                host,
                                dialect,
                            ),
                        )
                    })
                    .collect(),
            )
        }
        JsonValue::String(s) if s.contains(prefix) => {
            resolve_interpolation(s, namespaces, host, dialect)
        }
        JsonValue::Array(arr) => JsonValue::Array(
            arr.iter()
                .map(|v| interpolate_with_scope(v, namespaces, inheritable, scope, host, dialect))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Checks if `s` is exactly a single interpolation expression (nothing before or after).
/// Returns the inner expression text if so.
fn extract_single_expr<'a>(s: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let trimmed = s.strip_prefix(prefix)?;
    let end = trimmed.rfind(suffix)?;
    if end == trimmed.len() - suffix.len() {
        Some(&trimmed[..end])
    } else {
        None
    }
}

/// Replaces all interpolation patterns in a string, returning a `JsonValue`.
/// If the string is a single expression, the raw value is returned (preserving type).
/// Otherwise, all patterns are stringified and concatenated.
fn resolve_interpolation<H: Host<NonFunctionValue> + Clone + 'static>(
    s: &str,
    namespaces: &Namespaces,
    host: &H,
    dialect: &'static dyn Dialect,
) -> JsonValue {
    let prefix = dialect.syntax().interpolation_prefix();
    let suffix = dialect.syntax().interpolation_suffix();

    // Single expression → return raw value
    if let Some(inner) = extract_single_expr(s, prefix, suffix)
        && let Some(val) = eval_expression(inner, namespaces, host, dialect)
    {
        return val;
    }

    // Multiple expressions → stringify and concatenate
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find(prefix) {
        result.push_str(&remaining[..start]);
        let after_start = &remaining[start + prefix.len()..];
        if let Some(end) = after_start.find(suffix) {
            let expr = &after_start[..end];
            let val = eval_expression(expr, namespaces, host, dialect);
            match val {
                Some(JsonValue::String(t)) => result.push_str(&t),
                Some(JsonValue::Number(n)) => result.push_str(&n.to_string()),
                Some(JsonValue::Bool(b)) => result.push_str(&b.to_string()),
                Some(JsonValue::Null) => result.push_str("null"),
                Some(v) => result.push_str(&v.to_string()),
                None => result.push_str(&format!("{prefix}{expr}{suffix}")),
            }
            remaining = &after_start[end + suffix.len()..];
        } else {
            result.push_str(prefix);
            remaining = after_start;
        }
    }
    result.push_str(remaining);
    JsonValue::String(result)
}