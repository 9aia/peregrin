//! Capability checks — port of `functions/capability.ts`.
//!
//! A `capabilities` map gates functions twice: presence of any entry
//! whitelists the call at all (checked in `engine`), and in strict mode
//! the argument value must match the grant — `true` allows everything,
//! a list is glob alternatives, `{allow, deny}` is a glob policy.

use serde_json::Value;

use crate::context::Context;
use crate::error::TemplateError;
use crate::js::js_string;

/// `checkCapability` — only enforced under `ctx.strict`; denies when the
/// grant is missing/false or the value doesn't match the grant.
pub(crate) fn check_capability(
    name: &str,
    value: &str,
    ctx: &Context<'_>,
) -> Result<(), TemplateError> {
    if !ctx.strict {
        return Ok(());
    }
    let granted = ctx
        .capabilities
        .get(name)
        .is_some_and(|cap| capability_matches(value, cap));
    if granted {
        return Ok(());
    }
    Err(TemplateError::new(format!(
        "capability '{name}' not granted for '{value}'"
    )))
}

/// `matches` — `true` allows all; arrays glob-match; objects apply
/// `deny` then `allow` lists; anything else denies.
fn capability_matches(value: &str, caps: &Value) -> bool {
    match caps {
        Value::Bool(b) => *b,
        Value::Array(patterns) => patterns
            .iter()
            .any(|pattern| glob_match(&js_string(pattern), value)),
        Value::Object(map) => {
            for pattern in as_list(map.get("deny")) {
                if glob_match(pattern, value) {
                    return false;
                }
            }
            let allow = as_list(map.get("allow"));
            !allow.is_empty() && allow.iter().any(|pattern| glob_match(pattern, value))
        }
        // Null, numbers, strings — `matches` falls through to false.
        _ => false,
    }
}

/// `asList` — non-arrays yield `[]`; non-string items become `""`.
fn as_list(value: Option<&Value>) -> Vec<&str> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_str().unwrap_or(""))
            .collect(),
        _ => Vec::new(),
    }
}

/// `globMatch` — the TS builds `^...$` regex where `*` → `.*` (any run
/// except line terminators), `?` → `.` (one non-line-terminator char),
/// and every other metacharacter is escaped. Iterative matcher with
/// star backtracking; equivalent semantics without a regex dep.
fn glob_match(pattern: &str, value: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    let (mut p, mut v) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while v < value.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, v));
            p += 1;
        } else if p < pattern.len()
            && (pattern[p] == value[v] || (pattern[p] == '?' && !is_line_terminator(value[v])))
        {
            p += 1;
            v += 1;
        } else if let Some((star_p, star_v)) = star {
            if is_line_terminator(value[star_v]) {
                return false; // `.*` cannot cross a line terminator
            }
            star = Some((star_p, star_v + 1));
            p = star_p + 1;
            v = star_v + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// JS regex `.` does not match these.
fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn globs() {
        assert!(glob_match("echo *", "echo hello"));
        assert!(glob_match("*", ""));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(!glob_match("a?c", "axc\ndef")); // `?` and literals stay anchored
        assert!(!glob_match("a*c", "axc\nc")); // `*` cannot cross \n
        assert!(glob_match("a.c", "a.c")); // `.` is literal, not regex
        assert!(!glob_match("a.c", "abc"));
    }

    #[test]
    fn grants() {
        assert!(capability_matches("anything", &json!(true)));
        assert!(!capability_matches("anything", &json!(false)));
        assert!(!capability_matches("anything", &Value::Null));
        assert!(capability_matches("echo hi", &json!(["echo *"])));
        assert!(!capability_matches("rm x", &json!(["echo *"])));
        assert!(capability_matches("x", &json!({"allow": ["*"]})));
        assert!(!capability_matches(
            "x",
            &json!({"deny": ["*"], "allow": ["*"]})
        ));
        assert!(!capability_matches("x", &json!({"allow": []})));
        assert!(!capability_matches("x", &json!({})));
        assert!(!capability_matches("x", &json!(5)));
    }
}
