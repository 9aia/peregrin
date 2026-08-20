use ac_typecheck_core::Type;

/// A parsed expression AST — the intermediate representation for type inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprAst {
    /// A literal value with its inferred type.
    Literal(Type),
    /// A dotted path lookup (e.g., `doc.name`, `env.HOME`, `name`).
    Path(String),
    /// A function call: name + argument expressions.
    Call(String, Vec<ExprAst>),
}

/// Parse an expression string into an [`ExprAst`].
///
/// Returns `None` if the expression cannot be parsed.
pub fn parse_expr(s: &str) -> Option<ExprAst> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if s.starts_with("${") && s.ends_with('}') && s.len() > 3 {
        return parse_expr(&s[2..s.len() - 1]);
    }

    if let Some(paren_pos) = find_open_paren(s) {
        let name = s[..paren_pos].trim();
        let inner = &s[paren_pos + 1..s.len().saturating_sub(1)];
        let args = parse_args(inner)?;
        return Some(ExprAst::Call(name.to_string(), args));
    }

    if let Some(lit) = parse_literal(s) {
        return Some(lit);
    }

    Some(ExprAst::Path(s.to_string()))
}

/// Parse a literal value into an [`ExprAst::Literal`].
fn parse_literal(s: &str) -> Option<ExprAst> {
    let s = s.trim();

    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        return Some(ExprAst::Literal(Type::String));
    }

    if s.parse::<f64>().is_ok() {
        return Some(ExprAst::Literal(Type::Number));
    }

    if s == "true" || s == "false" {
        return Some(ExprAst::Literal(Type::Boolean));
    }

    if s == "null" {
        return Some(ExprAst::Literal(Type::Null));
    }

    None
}

/// Parse comma-separated arguments into a list of [`ExprAst`].
fn parse_args(s: &str) -> Option<Vec<ExprAst>> {
    let s = s.trim();
    if s.is_empty() {
        return Some(Vec::new());
    }

    let mut result = Vec::new();
    for arg in split_args(s) {
        let arg = arg.trim();
        let parsed = parse_expr(arg)?;
        result.push(parsed);
    }
    Some(result)
}

/// Find the position of the opening `(` for a function call.
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

/// Split function arguments by commas, respecting nested parens and string literals.
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
