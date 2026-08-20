use crate::path::resolve_path;
use crate::types::{FunctionResolver, Namespaces};
use serde_json::Value as JsonValue;

/// Evaluates a single expression to a `JsonValue`.
///
/// The expression uses the same syntax as the content inside `${...}`:
/// - Function calls: `add(1, 2)`, `mul(x, 2)`
/// - Path lookups: `doc.name`, `env.HOME`, `name` (shorthand for `doc.name`)
/// - Literals: `"string"`, `123`, `true`, `false`, `null`
///
/// Returns `None` if the expression cannot be resolved.
pub fn eval_expression(
    expr: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
) -> Option<JsonValue> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    eval_expr_inner(expr, namespaces, resolver)
}

/// Evaluates a user-defined function body expression.
///
/// Params are merged into the `doc` namespace (shadowing doc fields
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
) -> Option<JsonValue> {
    let mut merged = namespaces.clone();
    let mut doc_obj = merged
        .get("doc")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    for (name, value) in params {
        doc_obj.insert(name.clone(), value.clone());
    }
    merged.insert("doc".to_string(), JsonValue::Object(doc_obj));
    eval_expression(body, &merged, resolver)
}

/// Internal expression evaluator shared by `eval_expression` and interpolation.
pub(crate) fn eval_expr_inner(
    expr: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
) -> Option<JsonValue> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    // Function call: name(args)
    if let Some(paren_pos) = find_open_paren(expr) {
        let name = &expr[..paren_pos];
        let args_str = &expr[paren_pos + 1..expr.len().saturating_sub(1)];
        let args = parse_args(args_str, namespaces, resolver)?;
        resolver.call(name.trim(), &args).ok()
    } else {
        // Path lookup: ns.key.subkey or shorthand key
        resolve_path(expr, namespaces)
    }
}

/// Finds the position of the opening `(` for a function call.
/// Returns `None` if there is no `(` or it's inside a string literal.
fn find_open_paren(s: &str) -> Option<usize> {
    let mut in_string = false;
    let mut quote_char = '\0';
    for (i, ch) in s.char_indices() {
        if in_string {
            if ch == quote_char {
                in_string = false;
            }
        } else if ch == '"' || ch == '\'' {
            in_string = true;
            quote_char = ch;
        } else if ch == '(' {
            return Some(i);
        }
    }
    None
}

/// Parses function call arguments, evaluating each one.
fn parse_args(
    args_str: &str,
    namespaces: &Namespaces,
    resolver: &dyn FunctionResolver,
) -> Option<Vec<JsonValue>> {
    let args_str = args_str.trim();
    if args_str.is_empty() {
        return Some(Vec::new());
    }

    let mut args = Vec::new();
    for arg in split_args(args_str) {
        let arg = arg.trim();
        let val = eval_arg(arg, namespaces, resolver)?;
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
fn eval_arg(arg: &str, namespaces: &Namespaces, resolver: &dyn FunctionResolver) -> Option<JsonValue> {
    let arg = arg.trim();

    // Nested ${...}
    if arg.starts_with("${") && arg.ends_with('}') {
        let inner = &arg[2..arg.len() - 1];
        return eval_expr_inner(inner, namespaces, resolver);
    }

    // String literal
    if (arg.starts_with('"') && arg.ends_with('"') && arg.len() >= 2)
        || (arg.starts_with('\'') && arg.ends_with('\'') && arg.len() >= 2)
    {
        return Some(JsonValue::String(arg[1..arg.len() - 1].to_string()));
    }

    // Number literal
    if let Ok(n) = arg.parse::<f64>() {
        return serde_json::Number::from_f64(n).map(JsonValue::Number);
    }

    // Boolean literal
    if arg == "true" {
        return Some(JsonValue::Bool(true));
    }
    if arg == "false" {
        return Some(JsonValue::Bool(false));
    }
    if arg == "null" {
        return Some(JsonValue::Null);
    }

    // Nested function call
    if find_open_paren(arg).is_some() {
        return eval_expr_inner(arg, namespaces, resolver);
    }

    // Path lookup (namespace or shorthand)
    resolve_path(arg, namespaces)
}

/// Splits function arguments by commas, respecting nested parens and string literals.
fn split_args(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut quote_char = '\0';

    for ch in s.chars() {
        if in_string {
            current.push(ch);
            if ch == quote_char {
                in_string = false;
            }
        } else if ch == '"' || ch == '\'' {
            in_string = true;
            quote_char = ch;
            current.push(ch);
        } else if ch == '(' {
            depth += 1;
            current.push(ch);
        } else if ch == ')' {
            depth -= 1;
            current.push(ch);
        } else if ch == ',' && depth == 0 {
            result.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        result.push(current);
    }
    result
}
