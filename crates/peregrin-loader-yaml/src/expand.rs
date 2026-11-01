//! Expansion passes — port of `loader.ts`'s `deepMerge`,
//! `resolveIncludeDict`, `resolveIncludeStrings`, `resolveSnippet*`.
//!
//! All passes operate on untyped `serde_json::Value` trees, exactly like
//! the TS operates on plain JS objects. `seen` sets are the TS circular
//! guards: paths/names enter on descent and leave on the success path.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use peregrin_core::js_string;
use peregrin_loader::LoadError;
use serde_json::{Map, Value};

use crate::parse::{parse_yaml, read_text_file};

/// Scan `text` for `${name(inner)}` expressions — the `\$\{name\(([^)]+)\)\}`
/// regexes the TS uses. Returns `(start, end, inner)` byte ranges; the
/// inner group is `[^)]+` — non-empty, first `)` closes, `}` must
/// follow immediately. A failed candidate resumes scanning one byte
/// past its `${`, so nested markers match like the regex engine's.
fn scan_expressions<'a>(text: &'a str, name: &str) -> Vec<(usize, usize, &'a str)> {
    let marker = format!("${{{name}(");
    let mut matches = Vec::new();
    let mut cursor = 0;
    while let Some(pos) = text[cursor..].find(&marker) {
        let inner_start = cursor + pos + marker.len();
        let after = &text[inner_start..];
        let Some(close) = after.find(')') else {
            break;
        };
        if close == 0 || after.as_bytes().get(close + 1) != Some(&b'}') {
            cursor += pos + 1;
            continue;
        }
        let end = inner_start + close + 2;
        matches.push((cursor + pos, end, &text[inner_start..inner_start + close]));
        cursor = end;
    }
    matches
}

/// The trimmed `^${name(inner)}$` whole-string form — returns `inner`.
fn whole_expression<'a>(trimmed: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("${{{name}(");
    let inner = trimmed.strip_prefix(&marker)?.strip_suffix(")}")?;
    if inner.is_empty() || inner.contains(')') {
        return None;
    }
    Some(inner)
}

/// `path.dirname` — falls back to `"."` like node's dirname.
fn dirname(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}

/// `path.normalize` — lexical `..`/`.` collapsing; never touches the fs.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() && !out.has_root() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `path.isAbsolute ? p : path.normalize(path.join(base, p))`.
fn full_path(include_path: &str, base_dir: &Path) -> PathBuf {
    let candidate = Path::new(include_path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        normalize(&base_dir.join(candidate))
    }
}

/// TS `deepMerge(local, imported)` — local wins; dicts merge per-key,
/// arrays concatenate local-first, scalars take local.
pub(crate) fn deep_merge(local: &Value, imported: &Value) -> Value {
    match (local, imported) {
        (Value::Object(local_map), Value::Object(imported_map)) => {
            let mut result = imported_map.clone();
            for (key, value) in local_map {
                let merged = match result.get(key) {
                    Some(existing) => deep_merge(value, existing),
                    None => value.clone(),
                };
                result.insert(key.clone(), merged);
            }
            Value::Object(result)
        }
        (Value::Array(local_items), Value::Array(imported_items)) => {
            let mut out = local_items.clone();
            out.extend(imported_items.iter().cloned());
            Value::Array(out)
        }
        _ => local.clone(),
    }
}

// ---------------------------------------------------------------------------
// include: key expansion (resolveIncludeDict)
// ---------------------------------------------------------------------------

/// `resolveIncludeDict` — every dict value resolves depth-first, then a
/// dict-level `include:` key merges file contents *under* local keys.
/// The `seen` set guards circular includes; multiple `include:` paths
/// merge in order with earlier files winning.
pub(crate) fn resolve_include_dict(
    value: &Value,
    base_dir: &Path,
    seen: &mut HashSet<String>,
) -> Result<Value, LoadError> {
    match value {
        Value::Object(map) => {
            let mut resolved = Map::with_capacity(map.len());
            for (key, val) in map {
                resolved.insert(key.clone(), resolve_include_dict(val, base_dir, seen)?);
            }
            let Some(include_value) = resolved.remove("include") else {
                return Ok(Value::Object(resolved));
            };
            let paths = match include_value {
                Value::Array(items) => items,
                other => vec![other],
            };
            let mut merged = Value::Object(resolved);
            for include_path in paths {
                let Some(include_path) = include_path.as_str() else {
                    return Err(LoadError::new("include path must be a string"));
                };
                let full = full_path(include_path, base_dir);
                let key = full.to_string_lossy().into_owned();
                if seen.contains(&key) {
                    return Err(LoadError::new(format!("circular include detected: {key}")));
                }
                seen.insert(key.clone());
                let text = read_text_file(&full)?;
                let parsed = parse_yaml(&text)?;
                let imported = resolve_include_dict(&parsed, dirname(&full), seen)?;
                seen.remove(&key);
                merged = deep_merge(&merged, &imported);
            }
            Ok(merged)
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_include_dict(item, base_dir, seen)?);
            }
            Ok(Value::Array(out))
        }
        _ => Ok(value.clone()),
    }
}

// ---------------------------------------------------------------------------
// ${include(...)} string expansion (resolveIncludeStrings)
// ---------------------------------------------------------------------------

/// `resolveIncludeValue` — load+expand an included file for a
/// `${include(path)}` expression. Included content gets both
/// dict-include and string-include passes (anchored at the file's dir).
fn resolve_include_value(
    include_path: &str,
    base_dir: &Path,
    seen: &mut HashSet<String>,
) -> Result<Value, LoadError> {
    let full = full_path(include_path.trim(), base_dir);
    let key = full.to_string_lossy().into_owned();
    if seen.contains(&key) {
        return Err(LoadError::new(format!("circular include detected: {key}")));
    }
    seen.insert(key.clone());
    let text = read_text_file(&full)?;
    let parsed = parse_yaml(&text)?;
    let content = resolve_include_dict(&parsed, dirname(&full), seen)?;
    let content = resolve_include_strings(&content, dirname(&full), seen)?;
    seen.remove(&key);
    Ok(content)
}

/// `resolveIncludeStrings` — a string that *is* `${include(path)}`
/// (after trimming) yields the parsed file value typed; otherwise each
/// `${include(path)}` inside interpolates via `String(v)` — matches are
/// replaced right-to-left like the TS loop. Dict keys resolve too.
pub(crate) fn resolve_include_strings(
    value: &Value,
    base_dir: &Path,
    seen: &mut HashSet<String>,
) -> Result<Value, LoadError> {
    match value {
        Value::String(s) => {
            if let Some(inner) = whole_expression(s.trim(), "include") {
                return resolve_include_value(inner, base_dir, seen);
            }
            let matches = scan_expressions(s, "include");
            let mut result = s.clone();
            for (start, end, inner) in matches.into_iter().rev() {
                let val = resolve_include_value(inner, base_dir, seen)?;
                result.replace_range(start..end, &js_string(&val));
            }
            Ok(Value::String(result))
        }
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            for (key, val) in map {
                let resolved =
                    resolve_include_strings(&Value::String(key.clone()), base_dir, seen)?;
                let resolved_key = match resolved {
                    Value::String(k) => k,
                    other => js_string(&other),
                };
                out.insert(resolved_key, resolve_include_strings(val, base_dir, seen)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_include_strings(item, base_dir, seen)?);
            }
            Ok(Value::Array(out))
        }
        _ => Ok(value.clone()),
    }
}

// ---------------------------------------------------------------------------
// Snippet definition inheritance (resolveSnippetDefinitions)
// ---------------------------------------------------------------------------

/// `resolveSnippetDefinitions` — fixpoint pass over the `snippets:`
/// table resolving `snippet: <parent>` inheritance (local wins). Caps
/// at `len + 1` passes like the TS loop; a `b → a → b` chain surfaces
/// as a self-inheritance error when a resolved parent key lands back.
pub(crate) fn resolve_snippet_definitions(
    snippets: &Map<String, Value>,
) -> Result<Map<String, Value>, LoadError> {
    let mut resolved = snippets.clone();
    for _ in 0..=resolved.len() {
        let mut changed = false;
        // Object.entries() snapshot — parents resolve against the live map.
        let snapshot: Vec<(String, Value)> = resolved
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, body) in snapshot {
            let Some(obj) = body.as_object() else {
                continue;
            };
            if !obj.contains_key("snippet") {
                continue;
            }
            let mut local = obj.clone();
            let Some(parent_value) = local.remove("snippet") else {
                continue;
            };
            let Some(parent_name) = parent_value.as_str() else {
                return Err(LoadError::new(format!(
                    "snippet '{name}' references non-string parent"
                )));
            };
            let Some(parent) = resolved.get(parent_name) else {
                return Err(LoadError::new(format!(
                    "snippet '{name}' references unknown snippet '{parent_name}'"
                )));
            };
            if !parent.is_object() {
                return Err(LoadError::new(format!(
                    "snippet '{name}' inherits from non-dict '{parent_name}'"
                )));
            }
            if parent_name == name {
                return Err(LoadError::new(format!(
                    "circular snippet: '{name}' inherits from itself"
                )));
            }
            let merged = deep_merge(&Value::Object(local), parent);
            resolved.insert(name, merged);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    Ok(resolved)
}

// ---------------------------------------------------------------------------
// snippet: key expansion (resolveSnippetRefs)
// ---------------------------------------------------------------------------

/// `resolveSnippetRefs` — a dict's `snippet:` key merges the named
/// snippet body under local keys, then re-resolves recursively (with
/// the name held in `seen` so nested self-references fail).
pub(crate) fn resolve_snippet_refs(
    value: &Value,
    snippets: &Map<String, Value>,
    seen: &mut HashSet<String>,
) -> Result<Value, LoadError> {
    match value {
        Value::Object(map) => {
            let mut resolved = Map::with_capacity(map.len());
            for (key, val) in map {
                resolved.insert(key.clone(), resolve_snippet_refs(val, snippets, seen)?);
            }
            let Some(name_value) = resolved.remove("snippet") else {
                return Ok(Value::Object(resolved));
            };
            let Some(name) = name_value.as_str() else {
                return Err(LoadError::new("snippet key must be a string"));
            };
            let Some(body) = snippets.get(name) else {
                return Err(LoadError::new(format!("unknown snippet '{name}'")));
            };
            if seen.contains(name) {
                return Err(LoadError::new(format!(
                    "circular snippet reference: '{name}'"
                )));
            }
            seen.insert(name.to_string());
            if !body.is_object() {
                seen.remove(name);
                return Err(LoadError::new(format!("snippet '{name}' is not a dict")));
            }
            let merged = deep_merge(&Value::Object(resolved), body);
            let merged = resolve_snippet_refs(&merged, snippets, seen)?;
            seen.remove(name);
            Ok(merged)
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_snippet_refs(item, snippets, seen)?);
            }
            Ok(Value::Array(out))
        }
        _ => Ok(value.clone()),
    }
}

// ---------------------------------------------------------------------------
// ${snippet(...)} string expansion (resolveSnippetStrings)
// ---------------------------------------------------------------------------

/// `resolveSnippetValue` — a `${snippet(name)}` expression yields the
/// snippet body (typed when whole-match), with snippet strings inside
/// it resolved first.
fn resolve_snippet_value(
    name: &str,
    snippets: &Map<String, Value>,
    seen: &mut HashSet<String>,
) -> Result<Value, LoadError> {
    let name = name.trim();
    let Some(content) = snippets.get(name) else {
        return Err(LoadError::new(format!("unknown snippet '{name}'")));
    };
    if seen.contains(name) {
        return Err(LoadError::new(format!(
            "circular snippet reference: '{name}'"
        )));
    }
    seen.insert(name.to_string());
    let content = resolve_snippet_strings(content, snippets, seen)?;
    seen.remove(name);
    Ok(content)
}

/// `resolveSnippetStrings` — mirrors `resolveIncludeStrings` for the
/// `${snippet(name)}` form.
pub(crate) fn resolve_snippet_strings(
    value: &Value,
    snippets: &Map<String, Value>,
    seen: &mut HashSet<String>,
) -> Result<Value, LoadError> {
    match value {
        Value::String(s) => {
            if let Some(inner) = whole_expression(s.trim(), "snippet") {
                return resolve_snippet_value(inner, snippets, seen);
            }
            let matches = scan_expressions(s, "snippet");
            let mut result = s.clone();
            for (start, end, inner) in matches.into_iter().rev() {
                let val = resolve_snippet_value(inner, snippets, seen)?;
                result.replace_range(start..end, &js_string(&val));
            }
            Ok(Value::String(result))
        }
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            for (key, val) in map {
                let resolved =
                    resolve_snippet_strings(&Value::String(key.clone()), snippets, seen)?;
                let resolved_key = match resolved {
                    Value::String(k) => k,
                    other => js_string(&other),
                };
                out.insert(resolved_key, resolve_snippet_strings(val, snippets, seen)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_snippet_strings(item, snippets, seen)?);
            }
            Ok(Value::Array(out))
        }
        _ => Ok(value.clone()),
    }
}
