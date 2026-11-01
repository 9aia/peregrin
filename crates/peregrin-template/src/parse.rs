//! `${name.method(args)}` call parsing — port of `parse.ts`.
//!
//! Grammar (from the TS regex `^([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)?)\s*\(\s*(.*?)\s*\)\s*$`):
//! an identifier, an optional `.`-method identifier, optional whitespace,
//! `(`, anything, `)`, trailing whitespace. Args split on commas outside
//! quotes; each arg is coerced like the TS `parseArg`.

use serde_json::Value;

use crate::js::js_number_value;

/// A parsed `${name.method(args)}` call.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedCall {
    /// Function name (`env` in `${env("X")}`).
    pub(crate) name: String,
    /// Method selector (`unix` in `${now.unix()}`).
    pub(crate) method: Option<String>,
    /// Coerced arguments.
    pub(crate) args: Vec<Value>,
}

/// Parse placeholder inner text into a call, or `None` when the text
/// isn't a well-formed call (the TS `parseCall` `null` result — the
/// caller preserves the placeholder verbatim).
pub(crate) fn parse_call(content: &str) -> Option<ParsedCall> {
    let trimmed = content.trim();
    let name_len = ident_len(trimmed)?;
    let name = trimmed[..name_len].to_string();
    let mut rest = &trimmed[name_len..];
    let mut method = None;
    if let Some(after_dot) = rest.strip_prefix('.') {
        let method_len = ident_len(after_dot)?;
        method = Some(after_dot[..method_len].to_string());
        rest = &after_dot[method_len..];
    }
    let after_paren = rest.trim_start().strip_prefix('(')?;
    // `\)\s*$`: the closing paren is the last non-whitespace char.
    let args_text = after_paren.trim_end().strip_suffix(')')?;
    Some(ParsedCall {
        name,
        method,
        args: parse_args(args_text),
    })
}

/// `[a-zA-Z_]\w*` length in bytes — JS `\w` is ASCII-only.
fn ident_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let first = *bytes.first()?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut index = 1;
    while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
        index += 1;
    }
    Some(index)
}

/// Split on commas outside `'`/`"` quotes (no escape handling — a quote
/// only closes on the same character, matching the TS loop).
fn parse_args(text: &str) -> Vec<Value> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
        } else if ch == ',' {
            out.push(parse_arg(&current));
            current.clear();
        } else {
            current.push(ch);
        }
    }
    out.push(parse_arg(&current));
    out
}

/// Scalar coercion: quoted → string, `true`/`false`/`null`, JS number
/// literal → number, anything else → raw string.
fn parse_arg(text: &str) -> Value {
    let trimmed = text.trim();
    let quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''));
    if quoted {
        // A lone quote char (`"` or `'`) unquotes to "" (TS `slice(1, -1)`).
        let inner = if trimmed.len() >= 2 {
            &trimmed[1..trimmed.len() - 1]
        } else {
            ""
        };
        return Value::String(inner.to_string());
    }
    match trimmed {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ if !trimmed.is_empty() && is_number_literal(trimmed) => trimmed
            .parse::<f64>()
            .map_or_else(|_| Value::String(trimmed.to_string()), js_number_value),
        _ => Value::String(trimmed.to_string()),
    }
}

/// `-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?` over the whole string.
fn is_number_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = usize::from(bytes.first() == Some(&b'-'));
    let digits = |bytes: &[u8], index: &mut usize| {
        let start = *index;
        while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
            *index += 1;
        }
        *index > start
    };
    if !digits(bytes, &mut index) {
        return false;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        if !digits(bytes, &mut index) {
            return false;
        }
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        if !digits(bytes, &mut index) {
            return false;
        }
    }
    index == bytes.len()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_calls() {
        let call = parse_call("env( \"HOME\" , \"d\" )").unwrap();
        assert_eq!(call.name, "env");
        assert_eq!(call.method, None);
        assert_eq!(call.args, vec![json!("HOME"), json!("d")]);

        let call = parse_call("now.unix()").unwrap();
        assert_eq!(call.name, "now");
        assert_eq!(call.method.as_deref(), Some("unix"));
        assert!(call.args.is_empty());

        let call = parse_call("random(1, 10)").unwrap();
        assert_eq!(call.args, vec![json!(1), json!(10)]);
    }

    #[test]
    fn rejects_non_calls() {
        for text in [
            "config.region",
            "env",
            "env(",
            "env)",
            "1env()",
            "env.extra.more()",
            "",
            "a | b",
        ] {
            assert!(parse_call(text).is_none(), "{text}");
        }
    }

    #[test]
    fn arg_coercion() {
        assert_eq!(parse_arg("'x'"), json!("x"));
        assert_eq!(parse_arg("true"), json!(true));
        assert_eq!(parse_arg("null"), Value::Null);
        assert_eq!(parse_arg("-2.5e3"), json!(-2500));
        assert_eq!(parse_arg("raw"), json!("raw"));
        assert_eq!(parse_arg("\""), json!(""));
        // Unterminated quote → raw string (the `,` inside stays literal).
        let args = parse_args("\"a,b");
        assert_eq!(args, vec![json!("\"a,b")]);
    }
}
