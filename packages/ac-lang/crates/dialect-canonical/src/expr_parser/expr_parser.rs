use ac_dialect::ExprParser;

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
            _ => None,
        }
    }

    fn is_null_literal(&self, s: &str) -> bool {
        s.trim() == "null"
    }
}
