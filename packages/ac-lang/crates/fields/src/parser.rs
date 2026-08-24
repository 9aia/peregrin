use ac_runtime_values::NonFunctionValue;
use std::collections::HashSet;

use ac_ast_canonical::Expr;
use ac_dialect::Dialect;
use ac_lexer::Lexer;
use ac_lexer_canonical::CanonicalLexer;
use ac_parser::Parser;
use ac_parser_canonical::CanonicalParser;
use serde_json::Value as JsonValue;

use crate::types::Namespaces;

/// Parse an expression string into an [`Expr<NonFunctionValue>`] tree.
///
/// The resulting tree is evaluated by the VM, so path lookups are
/// encoded as host calls (`var` and `resolve`) and can be resolved
/// lazily against the current [`Context`].
pub fn parse_expr_to_tree(
    s: &str,
    dialect: &dyn Dialect,
    namespaces: &Namespaces,
) -> Option<Expr<NonFunctionValue>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let s = strip_interpolation(s, dialect);

    // Try the canonical lexer + parser for structured expressions
    // (calls, handles, arrays, objects, literals).
    if let Ok(tokens) = CanonicalLexer::default().lex(s) {
        if let Ok(expr) = CanonicalParser.parse(tokens) {
            return Some(expr);
        }
    }

    // Fallback: bare path lookup (e.g. `doc.name`, `env.HOME`, `name`)
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

fn build_path_expr(
    s: &str,
    dialect: &dyn Dialect,
    namespaces: &Namespaces,
) -> Expr<NonFunctionValue> {
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
    let path = JsonValue::Array(
        rest.iter()
            .map(|p| JsonValue::String(p.to_string()))
            .collect(),
    );
    let path_expr = Expr::value(NonFunctionValue::new(path));

    Expr::call("resolve", vec![root_expr, path_expr])
}