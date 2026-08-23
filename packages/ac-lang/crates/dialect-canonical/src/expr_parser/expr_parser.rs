use ac_dialect::ExprParser;
use ac_dialect::ParsedHandlerClause;

use crate::AcDialect;

impl ExprParser for AcDialect {
    fn find_call_open(&self, s: &str) -> Option<usize> {
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

    fn split_args(&self, s: &str) -> Vec<String> {
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
            } else if ch == '(' || ch == '[' {
                depth += 1;
                current.push(ch);
            } else if ch == ')' || ch == ']' {
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

    fn parse_string_literal(&self, s: &str) -> Option<String> {
        let s = s.trim();
        if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
            || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
        {
            Some(s[1..s.len() - 1].to_string())
        } else {
            None
        }
    }

    fn parse_number_literal(&self, s: &str) -> Option<f64> {
        s.trim().parse::<f64>().ok()
    }

    fn parse_bool_literal(&self, s: &str) -> Option<bool> {
        match s.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None}
    }

    fn is_null_literal(&self, s: &str) -> bool {
        s.trim() == "null"
    }

    fn find_handle_open(&self, s: &str) -> Option<usize> {
        let trimmed = s.trim_start();
        let leading = s.len() - trimmed.len();
        let keyword = "handle";
        if !trimmed.starts_with(keyword) {
            return None;
        }
        let after = &trimmed[keyword.len()..];
        // Must be followed by whitespace or '(' — not a longer identifier
        let next_ch = after.chars().next()?;
        if next_ch.is_alphanumeric() || next_ch == '_' {
            return None;
        }
        // Skip whitespace to find '('
        let rest = after.trim_start();
        if !rest.starts_with('(') {
            return None;
        }
        let paren_offset = trimmed.len() - rest.len();
        Some(leading + paren_offset)
    }

    fn find_block_open(&self, s: &str) -> Option<usize> {
        let mut in_string = false;
        let mut quote_char = '\0';
        let mut paren_depth = 0i32;
        for (i, ch) in s.char_indices() {
            if in_string {
                if ch == quote_char {
                    in_string = false;
                }
            } else if ch == '"' || ch == '\'' {
                in_string = true;
                quote_char = ch;
            } else if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth -= 1;
            } else if ch == '{' && paren_depth == 0 {
                return Some(i);
            }
        }
        None
    }

    fn split_handler_clauses(&self, s: &str) -> Vec<String> {
        let s = s.trim();
        if s.is_empty() {
            return Vec::new();
        }
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
            } else if ch == '(' || ch == '{' {
                depth += 1;
                current.push(ch);
            } else if ch == ')' || ch == '}' {
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

    fn parse_handler_clause(&self, s: &str) -> Option<ParsedHandlerClause> {
        let s = s.trim();

        // 1. Parse the operation name: "op_name"
        let op_end = self.op_name_end(s)?;
        let op_str = &s[..op_end];
        let op_name = self.parse_string_literal(op_str)?;
        let after_op = s[op_end..].trim_start();

        // 2. Parse params: (param1, param2) or ()
        if !after_op.starts_with('(') {
            return None;
        }
        let close = after_op.find(')')?;
        let params_str = &after_op[1..close];
        let params: Vec<String> = if params_str.trim().is_empty() {
            Vec::new()
        } else {
            params_str
                .split(',')
                .map(|p| p.trim().to_string())
                .collect()
        };

        // 3. Parse "as k"
        let after_params = after_op[close + 1..].trim_start();
        if !after_params.starts_with("as") {
            return None;
        }
        let after_as = &after_params[2..];
        // "as" must be followed by whitespace (not part of a longer identifier)
        let next_ch = after_as.chars().next();
        if next_ch.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        let after_as = after_as.trim_start();
        // The continuation name is the identifier up to the next whitespace + ':'
        // Find the colon that separates continuation name from body
        let colon_pos = after_as.find(':')?;
        let continuation_name = after_as[..colon_pos].trim().to_string();
        if continuation_name.is_empty() {
            return None;
        }

        // 4. The rest is the body expression string
        let body = after_as[colon_pos + 1..].trim().to_string();
        if body.is_empty() {
            return None;
        }

        Some(ParsedHandlerClause {
            op_name,
            params,
            continuation_name,
            body})
    }
}

impl AcDialect {
    /// Find the end of the leading string literal in `s`.
    /// Returns the byte offset just past the closing quote.
    fn op_name_end(&self, s: &str) -> Option<usize> {
        let leading = s.len() - s.trim_start().len();
        let trimmed = &s[leading..];
        if trimmed.is_empty() {
            return None;
        }
        let quote = trimmed.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let rest = &trimmed[1..];
        let close_idx = rest.find(quote)?;
        Some(leading + 1 + close_idx + 1)
    }
}
