use std::collections::HashSet;

use ac_ast_canonical::{Expr, HandlerClause};
use ac_dialect::Dialect;
use ac_runtime::NonFunctionValue;
use serde_json::{Number, Value as JsonValue};

use crate::types::Namespaces;

/// Parse an expression string into an [`Expr<NonFunctionValue>`] tree.
///
/// The resulting tree is evaluated by the VM, so path lookups are
/// encoded as host calls (`var` and `resolve`) and can be resolved
/// lazily against the current [`Context`].
pub fn parse_expr_to_tree(s: &str, dialect: &dyn Dialect, namespaces: &Namespaces) -> Option<Expr<NonFunctionValue>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let s = strip_interpolation(s, dialect);

    if let Some(paren_pos) = dialect.parser().find_handle_open(s) {
        return parse_handle(s, paren_pos, dialect, namespaces);
    }

    if let Some(paren_pos) = dialect.parser().find_call_open(s)
        && s.ends_with(dialect.syntax().call_close())
        && paren_pos < s.len() - 1
        && !s[..paren_pos].trim().is_empty()
    {
        return parse_call(s, paren_pos, dialect, namespaces);
    }

    if let Some(s_val) = dialect.parser().parse_string_literal(s) {
        return Some(Expr::value(NonFunctionValue::new(s_val)));
    }

    if let Some(n) = dialect.parser().parse_number_literal(s) {
        return Some(Expr::value(NonFunctionValue::new(parse_number_value(n))));
    }

    if let Some(b) = dialect.parser().parse_bool_literal(s) {
        return Some(Expr::value(NonFunctionValue::new(b)));
    }

    if dialect.parser().is_null_literal(s) {
        return Some(Expr::value(NonFunctionValue::new(JsonValue::Null)));
    }

    if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
        return parse_array(s, dialect, namespaces);
    }

    Some(build_path_expr(s, dialect, namespaces))
}

fn strip_interpolation<'a>(s: &'a str, dialect: &dyn Dialect) -> &'a str {
    let prefix = dialect.syntax().interpolation_prefix();
    let suffix = dialect.syntax().interpolation_suffix();
    if s.starts_with(prefix) && s.ends_with(suffix) && s.len() > prefix.len() + suffix.len() {
        s[prefix.len()..s.len() - suffix.len()].trim()
    } else {
        s
    }
}

fn parse_number_value(n: f64) -> JsonValue {
    if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        JsonValue::Number(Number::from(n as i64))
    } else {
        JsonValue::Number(Number::from_f64(n).unwrap_or_else(|| Number::from(0)))
    }
}

fn parse_array(s: &str, dialect: &dyn Dialect, namespaces: &Namespaces) -> Option<Expr<NonFunctionValue>> {
    let inner = &s[1..s.len() - 1];
    if inner.trim().is_empty() {
        return Some(Expr::call("array", Vec::new()));
    }
    let args: Vec<Expr<NonFunctionValue>> = dialect
        .parser()
        .split_args(inner)
        .iter()
        .filter_map(|arg| parse_expr_to_tree(arg.trim(), dialect, namespaces))
        .collect();
    Some(Expr::call("array", args))
}

fn parse_call(s: &str, paren_pos: usize, dialect: &dyn Dialect, namespaces: &Namespaces) -> Option<Expr<NonFunctionValue>> {
    let name = s[..paren_pos].trim();
    let inner = &s[paren_pos + 1..s.len() - 1];
    let args: Vec<Expr<NonFunctionValue>> = if inner.trim().is_empty() {
        Vec::new()
    } else {
        dialect
            .parser()
            .split_args(inner)
            .iter()
            .filter_map(|arg| parse_expr_to_tree(arg.trim(), dialect, namespaces))
            .collect()
    };
    Some(Expr::call(name, args))
}

fn parse_handle(s: &str, paren_pos: usize, dialect: &dyn Dialect, namespaces: &Namespaces) -> Option<Expr<NonFunctionValue>> {
    let inner_start = paren_pos + 1;
    let close_pos = find_matching_close(s, inner_start, dialect.syntax().call_open(), dialect.syntax().call_close())?;
    let body_str = &s[inner_start..close_pos];
    let body = parse_expr_to_tree(body_str, dialect, namespaces)?;

    let after_body = s[close_pos + 1..].trim_start();
    if !after_body.starts_with('{') || !s.ends_with('}') {
        return None;
    }

    let block_start = close_pos + 1 + (s[close_pos + 1..].len() - after_body.len());
    let block_inner = &s[block_start + 1..s.len() - 1];

    let handlers: Vec<HandlerClause<NonFunctionValue>> = dialect
        .parser()
        .split_handler_clauses(block_inner)
        .iter()
        .filter_map(|clause| {
            let parsed = dialect.parser().parse_handler_clause(clause)?;
            let body = parse_expr_to_tree(&parsed.body, dialect, namespaces)?;
            Some(HandlerClause {
                op_name: parsed.op_name,
                params: parsed.params,
                continuation_name: parsed.continuation_name,
                body,
            })
        })
        .collect();

    Some(Expr::handle(body, handlers))
}

fn build_path_expr(s: &str, dialect: &dyn Dialect, namespaces: &Namespaces) -> Expr<NonFunctionValue> {
    let sep = dialect.syntax().path_separator();
    let parts: Vec<&str> = s.split(sep).collect();
    let roots: HashSet<&str> = namespaces.keys().map(|k| k.as_str()).collect();

    let (root, rest) = if parts.is_empty() {
        ("doc", parts.as_slice())
    } else if roots.contains(parts[0]) {
        (parts[0], &parts[1..])
    } else {
        ("doc", parts.as_slice())
    };

    let root_expr = Expr::call("var", vec![Expr::value(NonFunctionValue::new(root))]);
    let path = JsonValue::Array(rest.iter().map(|p| JsonValue::String(p.to_string())).collect());
    let path_expr = Expr::value(NonFunctionValue::new(path));

    Expr::call("resolve", vec![root_expr, path_expr])
}

fn find_matching_close(s: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 1i32;
    let mut in_string = false;
    let mut quote_char = '\0';
    for (i, ch) in s[start..].char_indices() {
        if in_string {
            if ch == quote_char {
                in_string = false;
            }
        } else if ch == '"' || ch == '\'' {
            in_string = true;
            quote_char = ch;
        } else if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(start + i);
            }
        }
    }
    None
}
