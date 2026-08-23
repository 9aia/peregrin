use crate::expr_parser::ExprParser;

use crate::dialect::dialect_stub::StubDialect;

impl ExprParser for StubDialect {
    fn find_call_open(&self, s: &str) -> Option<usize> {
        let mut in_string: Option<char> = None;
        for (i, c) in s.char_indices() {
            match in_string {
                Some(quote) => {
                    if c == quote {
                        in_string = None;
                    }
                }
                None => {
                    if c == '"' || c == '\'' {
                        in_string = Some(c);
                    } else if c == '(' {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn split_args(&self, s: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0i32;
        let mut in_string: Option<char> = None;

        for c in s.chars() {
            match in_string {
                Some(quote) => {
                    current.push(c);
                    if c == quote {
                        in_string = None;
                    }
                }
                None => {
                    if c == '"' || c == '\'' {
                        in_string = Some(c);
                        current.push(c);
                    } else if c == '(' {
                        depth += 1;
                        current.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        current.push(c);
                    } else if c == ',' && depth == 0 {
                        args.push(current.trim().to_string());
                        current.clear();
                    } else {
                        current.push(c);
                    }
                }
            }
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() || !args.is_empty() {
            args.push(trimmed.to_string());
        }
        args
    }

    fn parse_string_literal(&self, s: &str) -> Option<String> {
        let trimmed = s.trim();
        if trimmed.len() >= 2
            && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        {
            Some(trimmed[1..trimmed.len() - 1].to_string())
        } else {
            None
        }
    }

    fn parse_number_literal(&self, s: &str) -> Option<f64> {
        s.trim().parse().ok()
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
}
