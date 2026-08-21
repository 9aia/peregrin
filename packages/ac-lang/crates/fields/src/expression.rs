use crate::path::resolve_path;
use crate::types::{APPLY_FUNCTION, DEFAULT_NAMESPACE, FunctionResolver, Namespaces};
use ac_dialect::Dialect;
use serde_json::Value as JsonValue;

/// Evaluates a single expression to a `JsonValue`.
///
/// The expression uses the syntax defined by the dialect:
/// - Function calls: `add(1, 2)`, `mul(x, 2)`
/// - Path lookups: `doc.name`, `env.HOME`, `name` (shorthand for `doc.name`)
/// - Literals: `"string"`, `123`, `true`, `false`, `null`
///
/// Returns `None` if the expression cannot be resolved.
pub fn eval_expression(
    expr: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
    dialect: &dyn Dialect,
) -> Option<JsonValue> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    eval_expr_inner(expr, namespaces, resolver, dialect)
}

/// Evaluates a user-defined function body expression.
///
/// Params are merged into the default namespace (shadowing fields
/// with the same name), so bare names like `x` resolve via shorthand.
/// The resolver is passed through, so the body can call host primitives
/// and other user-defined functions.
///
/// Returns `None` if the body cannot be evaluated.
pub fn eval_body(
    body: &str,
    params: &[(String, JsonValue)],
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
    dialect: &dyn Dialect,
) -> Option<JsonValue> {
    let mut merged = namespaces.clone();
    let mut doc_obj = merged
        .get(DEFAULT_NAMESPACE)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    for (name, value) in params {
        doc_obj.insert(name.clone(), value.clone());
    }
    merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));
    eval_expression(body, &merged, resolver, dialect)
}

/// Internal expression evaluator shared by `eval_expression` and interpolation.
pub(crate) fn eval_expr_inner(
    expr: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
    dialect: &dyn Dialect,
) -> Option<JsonValue> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    // Function call: name(args)
    if let Some(paren_pos) = dialect.parser().find_call_open(expr)
        && expr.ends_with(dialect.syntax().call_close())
        && paren_pos < expr.len() - 1
        && !expr[..paren_pos].trim().is_empty()
    {
        let name = &expr[..paren_pos];
        let args_str = &expr[paren_pos + 1..expr.len() - 1];
        let args = parse_args(args_str, namespaces, resolver, dialect)?;

        let name_trimmed = name.trim();

        // Try resolving the name as a path. If it resolves to a
        // function value, dispatch via `apply`. If it resolves to
        // a non-function (e.g. a field named "import" that holds
        // a string), apply fails and we fall through to the host
        // function call. If it doesn't resolve at all, fall through.
        if let Some(val) = resolve_path(name_trimmed, namespaces, dialect.syntax().path_separator())
        {
            let mut apply_args = vec![val];
            apply_args.extend(args.iter().cloned());
            if let Ok(result) = resolver.call(APPLY_FUNCTION, &apply_args) {
                return Some(result);
            }
        }

        resolver.call(name_trimmed, &args).ok()
    } else {
        // Path lookup: ns.key.subkey or shorthand key
        resolve_path(expr, namespaces, dialect.syntax().path_separator())
    }
}

/// Parses function call arguments, evaluating each one.
fn parse_args(
    args_str: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
    dialect: &dyn Dialect,
) -> Option<Vec<JsonValue>> {
    let args_str = args_str.trim();
    if args_str.is_empty() {
        return Some(Vec::new());
    }

    let mut args = Vec::new();
    for arg in dialect.parser().split_args(args_str) {
        let arg = arg.trim();
        let val = eval_arg(arg, namespaces, resolver, dialect)?;
        args.push(val);
    }
    Some(args)
}

/// Evaluates a single argument value.
///
/// - `"literal"` → string literal
/// - `123` / `1.5` → number literal
/// - `true` / `false` → boolean literal
/// - `${...}` → nested interpolation (recursive)
/// - `ns.path` or `shorthand` → path lookup
fn eval_arg(
    arg: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
    dialect: &dyn Dialect,
) -> Option<JsonValue> {
    let arg = arg.trim();

    // Nested interpolation (e.g. ${...} for canonical dialect)
    let prefix = dialect.syntax().interpolation_prefix();
    let suffix = dialect.syntax().interpolation_suffix();
    if arg.starts_with(prefix) && arg.ends_with(suffix) && arg.len() > prefix.len() + suffix.len() {
        let inner = &arg[prefix.len()..arg.len() - suffix.len()];
        return eval_expr_inner(inner, namespaces, resolver, dialect);
    }

    // String literal
    if let Some(s) = dialect.parser().parse_string_literal(arg) {
        return Some(JsonValue::String(s));
    }

    // Number literal
    if let Some(n) = dialect.parser().parse_number_literal(arg) {
        return serde_json::Number::from_f64(n).map(JsonValue::Number);
    }

    // Boolean literal
    if let Some(b) = dialect.parser().parse_bool_literal(arg) {
        return Some(JsonValue::Bool(b));
    }

    // Null literal
    if dialect.parser().is_null_literal(arg) {
        return Some(JsonValue::Null);
    }

    // Nested function call
    if dialect.parser().find_call_open(arg).is_some() {
        return eval_expr_inner(arg, namespaces, resolver, dialect);
    }

    // Path lookup (namespace or shorthand)
    resolve_path(arg, namespaces, dialect.syntax().path_separator())
}
