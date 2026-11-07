//! Capability grant matching — the `capabilities.<name>` value
//! forms shared by the template engine's `checkCapability` gate, the
//! driver's secret-source gate (`declared_only` stores), and driver
//! impls that enforce `capabilities.secrets` themselves. `true`
//! allows everything; an array is glob alternatives; `{allow, deny}`
//! applies the deny list first, then requires an allow hit; anything
//! else denies.

use serde_json::Value;

use crate::js_string;

/// `capabilityMatches` — does `value` satisfy the `grant` entry?
#[must_use]
pub fn capability_matches(value: &str, grant: &Value) -> bool {
    match grant {
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
            .map(|item| item.as_str().unwrap_or_default())
            .collect(),
        _ => Vec::new(),
    }
}

/// `globMatch` — `^…$`-anchored glob where `*` → `.*` (any run
/// except line terminators), `?` → `.` (one non-line-terminator
/// char), and every other metacharacter is literal. Iterative
/// matcher with star backtracking — equivalent to the TS's
/// constructed regex without a regex dependency.
#[must_use]
pub fn glob_match(pattern: &str, value: &str) -> bool {
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
