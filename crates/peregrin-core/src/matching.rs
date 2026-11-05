//! Dot-separated event-type matching — port of `bus.ts`'s
//! `matchPattern`/`matches`/`captureWildcards`. `*` captures one
//! segment, `**` captures zero or more (greedy, first match wins).
//! Captures are 1-indexed; `shard` aliases the first capture.

use std::collections::HashMap;

/// The recursive matcher — `step(pi, ei, caps)` in the TS.
fn step(
    p: &[&str],
    e: &[&str],
    pi: usize,
    ei: usize,
    caps: Vec<Vec<String>>,
) -> Option<Vec<Vec<String>>> {
    if pi == p.len() {
        return (ei == e.len()).then_some(caps);
    }
    match p[pi] {
        "*" => {
            if ei >= e.len() {
                return None;
            }
            let mut caps = caps;
            caps.push(vec![e[ei].to_string()]);
            step(p, e, pi + 1, ei + 1, caps)
        }
        "**" => {
            for k in 0..=e.len() - ei {
                let mut caps = caps.clone();
                caps.push(e[ei..ei + k].iter().map(|s| (*s).to_string()).collect());
                if let Some(result) = step(p, e, pi + 1, ei + k, caps) {
                    return Some(result);
                }
            }
            None
        }
        part => {
            if ei >= e.len() || e[ei] != part {
                return None;
            }
            step(p, e, pi + 1, ei + 1, caps)
        }
    }
}

/// `matchPattern` — returns the captured segments per wildcard, or
/// `None` when `pattern` doesn't match `kind`. Each `*` produces a
/// one-segment capture, `**` a (possibly empty) multi-segment capture.
fn match_pattern(pattern: &str, kind: &str) -> Option<Vec<Vec<String>>> {
    let p: Vec<&str> = pattern.split('.').collect();
    let e: Vec<&str> = kind.split('.').collect();
    step(&p, &e, 0, 0, Vec::new())
}

/// `matches` — does `pattern` match the event type `kind`?
#[must_use]
pub fn matches(pattern: &str, kind: &str) -> bool {
    match_pattern(pattern, kind).is_some()
}

/// Could a subscription `pattern` be satisfied by a node declaring
/// `emit` as an emitted type? `emit` is the *declared base* — the
/// engine expands a templated emit (`params.emit`) with capture
/// segments at runtime (`push.gated` → `push.gated.<repo>.<url>.<sha>`),
/// so a pattern that extends `emit` with further segments counts too.
/// The wiring validator (`campaign validate`, the MCP tool) uses this to
/// avoid false "nothing emits it" warnings on routed/typed emits.
#[must_use]
pub fn subscribes_to(pattern: &str, emit: &str) -> bool {
    matches(pattern, emit) || pattern.starts_with(&format!("{emit}."))
}

/// `captureWildcards` — `{"1": …, "2": …}` per wildcard plus `shard`
/// for the first; `**` captures join back with `.`.
#[must_use]
pub fn capture_wildcards(pattern: &str, kind: &str) -> Option<HashMap<String, String>> {
    let result = match_pattern(pattern, kind)?;
    let mut caps = HashMap::new();
    for (i, cap) in result.iter().enumerate() {
        caps.insert((i + 1).to_string(), cap.join("."));
    }
    if let Some(first) = result.first() {
        caps.insert("shard".to_string(), first.join("."));
    }
    Some(caps)
}
