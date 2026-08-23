use ac_dialect::Dialect;
use ac_typecheck_core::Type;

/// A parsed handler clause for type inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerClauseAst {
    /// The operation name to match (e.g. `"fetch"`).
    pub op_name: String,

    /// Parameter names bound to the operation's arguments.
    pub params: Vec<String>,

    /// The name bound to the captured continuation.
    pub continuation_name: String,

    /// The handler body expression.
    pub body: Box<ExprAst>}

/// A parsed expression AST — the intermediate representation for type inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprAst {
    /// A literal value with its inferred type.
    Literal(Type),
    /// A path lookup (e.g., `doc.name`, `env.HOME`, `name`).
    Path(String),
    /// A function call: name + argument expressions.
    Call(String, Vec<ExprAst>),
    /// A handle expression: body + handler clauses.
    Handle {
        body: Box<ExprAst>,
        handlers: Vec<HandlerClauseAst>}}

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

    // Handle expression: handle(body) { clauses }
    if let Some(paren_pos) = dialect.parser().find_handle_open(s) {
        let inner_start = paren_pos + 1;
        let close_pos = find_matching_close(s, inner_start, '(', ')')?;
        let body_str = &s[inner_start..close_pos];

        let after_body = s[close_pos + 1..].trim_start();
        if !after_body.starts_with('{') {
            return None;
        }
        let block_start = close_pos + 1 + (s[close_pos + 1..].len() - after_body.len());
        if !s.ends_with('}') {
            return None;
        }
        let block_inner = &s[block_start + 1..s.len() - 1];

        let body = Box::new(parse_expr(body_str, dialect)?);
        let handlers: Vec<HandlerClauseAst> = dialect
            .parser()
            .split_handler_clauses(block_inner)
            .iter()
            .filter_map(|clause| {
                let parsed = dialect.parser().parse_handler_clause(clause)?;
                let body = Box::new(parse_expr(&parsed.body, dialect)?);
                Some(HandlerClauseAst {
                    op_name: parsed.op_name,
                    params: parsed.params,
                    continuation_name: parsed.continuation_name,
                    body})
            })
            .collect();

        return Some(ExprAst::Handle { body, handlers });
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

/// Find the matching close delimiter for the open delimiter at `start`,
/// respecting nested delimiters and string literals.
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
