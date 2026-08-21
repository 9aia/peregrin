use ac_dialect::Dialect;
use ac_typecheck_core::Type;

/// A parsed expression AST — the intermediate representation for type inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprAst {
    /// A literal value with its inferred type.
    Literal(Type),
    /// A path lookup (e.g., `doc.name`, `env.HOME`, `name`).
    Path(String),
    /// A function call: name + argument expressions.
    Call(String, Vec<ExprAst>),
}

/// Parse an expression string into an [`ExprAst`].
///
/// Returns `None` if the expression cannot be parsed.
pub fn parse_expr(s: &str, dialect: &dyn Dialect) -> Option<ExprAst> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let prefix = dialect.syntax().interpolation_prefix();
    let suffix = dialect.syntax().interpolation_suffix();
    if s.starts_with(prefix) && s.ends_with(suffix) && s.len() > prefix.len() + suffix.len() {
        return parse_expr(&s[prefix.len()..s.len() - suffix.len()], dialect);
    }

    if let Some(paren_pos) = dialect.parser().find_call_open(s)
        && s.ends_with(dialect.syntax().call_close())
        && paren_pos < s.len() - 1
        && !s[..paren_pos].trim().is_empty()
    {
        let name = s[..paren_pos].trim();
        let inner = &s[paren_pos + 1..s.len() - 1];
        let args = parse_args(inner, dialect)?;
        return Some(ExprAst::Call(name.to_string(), args));
    }

    if let Some(lit) = parse_literal(s, dialect) {
        return Some(lit);
    }

    Some(ExprAst::Path(s.to_string()))
}

/// Parse a literal value into an [`ExprAst::Literal`].
fn parse_literal(s: &str, dialect: &dyn Dialect) -> Option<ExprAst> {
    let s = s.trim();

    if dialect.parser().parse_string_literal(s).is_some() {
        return Some(ExprAst::Literal(Type::String));
    }

    if dialect.parser().parse_number_literal(s).is_some() {
        return Some(ExprAst::Literal(Type::Number));
    }

    if dialect.parser().parse_bool_literal(s).is_some() {
        return Some(ExprAst::Literal(Type::Boolean));
    }

    if dialect.parser().is_null_literal(s) {
        return Some(ExprAst::Literal(Type::Null));
    }

    None
}

/// Parse comma-separated arguments into a list of [`ExprAst`].
fn parse_args(s: &str, dialect: &dyn Dialect) -> Option<Vec<ExprAst>> {
    let s = s.trim();
    if s.is_empty() {
        return Some(Vec::new());
    }

    let mut result = Vec::new();
    for arg in dialect.parser().split_args(s) {
        let arg = arg.trim();
        let parsed = parse_expr(arg, dialect)?;
        result.push(parsed);
    }
    Some(result)
}
