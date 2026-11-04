//! The resolver — port of `engine.ts` plus the scope-resolution pass
//! (`docs/reference/scope-resolution.md`).
//!
//! Four stages: default normalization → function resolution (the
//! `/\$\{([^}]+)\}/g` render scan — placeholders split on the *first*
//! `}`, no nesting) → scope resolution (a nested-brace scan so `:-`
//! defaults containing placeholders stay whole) → strict-mode audit.

use serde_json::{Map, Value};

use crate::capability::check_capability;
use crate::context::{Context, Options};
use crate::error::TemplateError;
use crate::function::{TemplateFunction, builtins};
use crate::js::to_string_safe;
use crate::parse::parse_call;
use crate::scope::{EventScope, ParamsScope};

/// `${file.x}`-style dotted forms under the I/O builtin names still
/// defer past the scope pass — the strict audit excuses them.
const DEFERRED_PREFIXES: &[&str] = &["file", "shell", "env", "random", "uuid", "now"];

/// Cap on `:-` default chains — a nested default re-runs the whole
/// pipeline; deeper than this the expression counts as a miss.
const MAX_DEFAULT_DEPTH: u32 = 8;

/// `resolveTemplate` — render `${...}` placeholders, preserving anything
/// unrecognized; in strict mode fail on what remains unresolved.
///
/// The TS signature took an unused `triggers` parameter; it is dropped
/// here. `options` carries the optional `node`, custom functions, and
/// the [`Scope`](crate::Scope) bindings for the scope-resolution pass.
pub fn resolve_template(template: &str, ctx: &Context<'_>) -> Result<String, TemplateError> {
    resolve_template_opts(template, ctx, &Options::default())
}

/// `resolveTemplate` with `node` / `customFunctions` / `scope`.
pub fn resolve_template_opts(
    template: &str,
    ctx: &Context<'_>,
    options: &Options<'_>,
) -> Result<String, TemplateError> {
    let functions = all_functions(options.functions);
    let rendered = resolve_pipeline(template, ctx, options, &functions, 0)?;
    if ctx.strict {
        let unresolved = find_all_placeholders(&rendered, ctx, &functions);
        if !unresolved.is_empty() {
            return Err(TemplateError::new(format!(
                "unresolved placeholders: {}",
                unresolved.join(", ")
            )));
        }
        let missing = ctx.missing_secrets.borrow();
        if !missing.is_empty() {
            return Err(TemplateError::new(format!(
                "missing secrets: {}",
                missing.join(", ")
            )));
        }
    }
    Ok(rendered)
}

/// Stages 1–3 of the pipeline — default normalization, function
/// resolution, scope resolution. The strict audit (stage 4) only runs
/// once at the top of [`resolve_template_opts`]; `depth` counts chained
/// `:-` default re-resolutions and caps at [`MAX_DEFAULT_DEPTH`].
fn resolve_pipeline(
    template: &str,
    ctx: &Context<'_>,
    options: &Options<'_>,
    functions: &[TemplateFunction],
    depth: u32,
) -> Result<String, TemplateError> {
    let pre = resolve_defaults(template);
    let rendered = render_template(&pre, ctx, options.node, functions)?;
    render_scope(&rendered, ctx, options, functions, depth)
}

/// `resolveValue` — recurse into strings/arrays/objects, resolving each
/// string as a template.
pub fn resolve_value(value: &Value, ctx: &Context<'_>) -> Result<Value, TemplateError> {
    resolve_value_opts(value, ctx, &Options::default())
}

/// `resolveValueForNode` — `resolveValue` with the owning node.
pub fn resolve_value_for_node(
    value: &Value,
    ctx: &Context<'_>,
    node: &peregrin_core::Node,
) -> Result<Value, TemplateError> {
    resolve_value_opts(
        value,
        ctx,
        &Options {
            node: Some(node),
            functions: &[],
            scope: crate::scope::Scope::default(),
        },
    )
}

/// `resolveValue` with `node` / `customFunctions`.
pub fn resolve_value_opts(
    value: &Value,
    ctx: &Context<'_>,
    options: &Options<'_>,
) -> Result<Value, TemplateError> {
    match value {
        Value::String(text) => Ok(Value::String(resolve_template_opts(text, ctx, options)?)),
        Value::Array(items) => items
            .iter()
            .map(|item| resolve_value_opts(item, ctx, options))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, item) in map {
                out.insert(key.clone(), resolve_value_opts(item, ctx, options)?);
            }
            Ok(Value::Object(out))
        }
        _ => Ok(value.clone()),
    }
}

/// Resolve a declared `params:` map — when `options.scope.params` is
/// [`ParamsScope::Declared`], each key resolves through the same
/// mark/memoize path `${params.*}` lookups use, so cross-references and
/// the resolved map agree on one value per key per pass and circular
/// references miss. Any other `params` binding (or none) resolves each
/// value plainly, like [`resolve_value_opts`].
///
/// # Errors
/// [`TemplateError`] from any nested resolution.
pub fn resolve_params(
    value: &Value,
    ctx: &Context<'_>,
    options: &Options<'_>,
) -> Result<Value, TemplateError> {
    let (Value::Object(map), ParamsScope::Declared(state)) = (value, &options.scope.params) else {
        return resolve_value_opts(value, ctx, options);
    };
    let mut out = Map::new();
    for (key, raw) in map {
        let resolved = resolve_declared_key(key, raw, state, ctx, options)?;
        out.insert(key.clone(), resolved);
    }
    Ok(Value::Object(out))
}

/// `{...templateFunctions, ...customFunctions}` — custom wins on name
/// collision.
fn all_functions(custom: &[TemplateFunction]) -> Vec<TemplateFunction> {
    let mut functions = builtins();
    for f in custom {
        match functions.iter().position(|b| b.name == f.name) {
            Some(index) => functions[index] = f.clone(),
            None => functions.push(f.clone()),
        }
    }
    functions
}

/// `findMatchingBrace` — index of the `}` closing the `{` at `start`;
/// depth may go negative mid-scan like the TS counter.
fn find_matching_brace(text: &str, start: usize) -> Option<usize> {
    let mut depth: i32 = 0;
    for (index, byte) in text.as_bytes().iter().enumerate().skip(start) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// `isJinjaExpr` — a placeholder counts as deferred-to-a-later-pass
/// when it's a bare I/O-builtin prefix or `prefix.`/`prefix(` path with
/// no `|` filter. Scope-namespace leftovers are *not* deferred — the
/// scope pass already ran, so they count as unresolvable.
fn is_deferred_expr(content: &str) -> bool {
    let stripped = content.trim();
    if stripped.contains('|') {
        return false;
    }
    DEFERRED_PREFIXES.iter().any(|prefix| {
        stripped == *prefix
            || stripped
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('.') || rest.starts_with('('))
    })
}

/// `resolveDefaults` — normalize `${ VAR :- default }` whitespace and
/// recurse into defaults; the pass does *not* resolve defaults, it just
/// rewrites them so the render pass sees canonical `${VAR:-...}` text.
fn resolve_defaults(input: &str) -> String {
    let mut result = String::new();
    let mut index = 0;
    let bytes = input.as_bytes();
    while index < input.len() {
        if bytes[index] == b'$' && index + 1 < input.len() && bytes[index + 1] == b'{' {
            match find_matching_brace(input, index + 1) {
                None => {
                    result.push_str(&input[index..]);
                    break;
                }
                Some(end) => {
                    let inner = &input[index + 2..end];
                    if let Some(sep) = inner.find(":-") {
                        let var_part = inner[..sep].trim();
                        let default_part = &inner[sep + 2..];
                        result.push_str("${");
                        result.push_str(var_part);
                        result.push_str(":-");
                        result.push_str(&resolve_defaults(default_part));
                        result.push('}');
                    } else {
                        result.push_str(&input[index..=end]);
                    }
                    index = end + 1;
                }
            }
        } else {
            // Copy one char — may be multibyte.
            match input[index..].chars().next() {
                Some(ch) => {
                    result.push(ch);
                    index += ch.len_utf8();
                }
                None => break,
            }
        }
    }
    result
}

/// The render-scan gate: `caps[name]` defined and not `false`.
fn cap_gate(ctx: &Context<'_>, name: &str) -> bool {
    !matches!(ctx.capabilities.get(name), None | Some(Value::Bool(false)))
}

/// `resolvePlaceholderEffect` — resolve one `${...}`; preserve verbatim
/// when it isn't a call, the function is unknown, or it isn't granted.
fn resolve_placeholder(
    placeholder: &str,
    ctx: &Context<'_>,
    functions: &[TemplateFunction],
    node: Option<&peregrin_core::Node>,
) -> Result<String, TemplateError> {
    if !(placeholder.starts_with("${") && placeholder.ends_with('}')) {
        return Ok(placeholder.to_string());
    }
    let content = &placeholder[2..placeholder.len() - 1];
    let Some(call) = parse_call(content) else {
        return Ok(placeholder.to_string());
    };
    let Some(function) = functions.iter().find(|f| f.name == call.name) else {
        return Ok(placeholder.to_string());
    };
    if !cap_gate(ctx, &call.name) {
        return Ok(placeholder.to_string());
    }
    let value = function.call(&call.args, ctx, node, call.method.as_deref())?;
    Ok(to_string_safe(&value))
}

/// `renderTemplateEffect` — the `/\$\{([^}]+)\}/g` pass: placeholders
/// end at the first `}` (no nesting) and need a non-empty interior.
fn render_template(
    input: &str,
    ctx: &Context<'_>,
    node: Option<&peregrin_core::Node>,
    functions: &[TemplateFunction],
) -> Result<String, TemplateError> {
    let mut out = String::new();
    let mut last = 0;
    let mut index = 0;
    let bytes = input.as_bytes();
    while index + 1 < bytes.len() {
        if bytes[index] == b'$' && bytes[index + 1] == b'{' {
            let Some(rel) = input[index + 2..].find('}') else {
                break;
            };
            let end = index + 2 + rel;
            if rel > 0 {
                out.push_str(&input[last..index]);
                out.push_str(&resolve_placeholder(
                    &input[index..=end],
                    ctx,
                    functions,
                    node,
                )?);
                last = end + 1;
            }
            index = end + 1;
        } else {
            index += 1;
        }
    }
    out.push_str(&input[last..]);
    Ok(out)
}

/// `isUnresolvable` — after the scope pass, would this placeholder have
/// resolved? Calls need a granted known function; anything else needs to
/// look like a deferred I/O-builtin expression.
fn is_unresolvable(content: &str, ctx: &Context<'_>, functions: &[TemplateFunction]) -> bool {
    if let Some(call) = parse_call(content) {
        let gated = functions.iter().any(|f| f.name == call.name) && cap_gate(ctx, &call.name);
        return !gated;
    }
    !is_deferred_expr(content)
}

/// `findAllPlaceholders` — the strict-mode audit scan; unlike the render
/// pass it counts nested braces (`findMatchingBrace`).
fn find_all_placeholders(
    input: &str,
    ctx: &Context<'_>,
    functions: &[TemplateFunction],
) -> Vec<String> {
    let mut found = Vec::new();
    let mut index = 0;
    let bytes = input.as_bytes();
    while index < input.len() {
        if bytes[index] == b'$' && index + 1 < input.len() && bytes[index + 1] == b'{' {
            match find_matching_brace(input, index + 1) {
                None => {
                    found.push(input[index..].to_string());
                    break;
                }
                Some(end) => {
                    let inner = input[index + 2..end].trim();
                    if is_unresolvable(inner, ctx, functions) {
                        found.push(input[index..=end].to_string());
                    }
                    index = end + 1;
                }
            }
        } else {
            index += 1;
        }
    }
    found
}

// ---------------------------------------------------------------------
// Stage 3 — scope resolution (`docs/reference/scope-resolution.md`)
// ---------------------------------------------------------------------

/// Re-scan the rendered text for `${ns.path[:-default]}` scope
/// placeholders. Unlike the render pass this scan counts nested braces
/// (`find_matching_brace`) so defaults containing placeholders are
/// consumed whole. Hits substitute; misses emit verbatim — or the
/// `:-` default, re-resolved through the full pipeline.
fn render_scope(
    input: &str,
    ctx: &Context<'_>,
    options: &Options<'_>,
    functions: &[TemplateFunction],
    depth: u32,
) -> Result<String, TemplateError> {
    let mut out = String::new();
    let mut last = 0;
    let mut index = 0;
    let bytes = input.as_bytes();
    while index < input.len() {
        if bytes[index] == b'$' && index + 1 < input.len() && bytes[index + 1] == b'{' {
            match find_matching_brace(input, index + 1) {
                None => break,
                Some(end) => {
                    let content = &input[index + 2..end];
                    let resolved = resolve_scope_expr(content, ctx, options, functions, depth)?;
                    out.push_str(&input[last..index]);
                    match resolved {
                        Some(text) => out.push_str(&text),
                        None => out.push_str(&input[index..=end]),
                    }
                    last = end + 1;
                    index = end + 1;
                }
            }
        } else {
            index += 1;
        }
    }
    out.push_str(&input[last..]);
    Ok(out)
}

/// Resolve one `${...}` interior as a scope expression — `Some(text)`
/// substitutes, `None` preserves the placeholder verbatim. Anything
/// that isn't `ns.segment…` over the six scope namespaces — calls,
/// `|` filters, bare namespaces, empty segments, foreign heads — is
/// not a scope expression and is left alone.
fn resolve_scope_expr(
    content: &str,
    ctx: &Context<'_>,
    options: &Options<'_>,
    functions: &[TemplateFunction],
    depth: u32,
) -> Result<Option<String>, TemplateError> {
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.contains('|') {
        return Ok(None);
    }
    let (head, default) = match trimmed.find(":-") {
        Some(sep) => (&trimmed[..sep], Some(&trimmed[sep + 2..])),
        None => (trimmed, None),
    };
    let head = head.trim_end();
    if parse_call(head).is_some() {
        return Ok(None);
    }
    // `secret` is the exception: everything up to `:-` is one key —
    // secret names may contain dots.
    if let Some(name) = head.strip_prefix("secret.") {
        if name.is_empty() {
            return Ok(None);
        }
        return resolve_secret(name, default, ctx, options, functions, depth);
    }
    let mut parts = head.split('.');
    let ns = parts.next().unwrap_or_default();
    let segments: Vec<&str> = parts.collect();
    if segments.is_empty() || segments.iter().any(|s| s.is_empty()) {
        return Ok(None);
    }
    // Foreign namespaces (`env.PATH`, `HOME`, …) are not scope
    // expressions — left alone, `:-` default intact.
    let value = match ns {
        "event" => lookup_event(&segments, options.scope.event.as_ref()),
        "config" => options
            .scope
            .config
            .and_then(|config| config.get(segments[0]))
            .and_then(|value| walk(value, &segments[1..])),
        "params" => lookup_params(&segments, ctx, options)?,
        "state" => options
            .scope
            .state
            .and_then(|state| state.get(segments[0]))
            .and_then(|record| walk(&record, &segments[1..])),
        "node" => lookup_node(&segments, options.scope.node.as_ref()),
        _ => return Ok(None),
    };
    match value {
        Some(value) => Ok(Some(to_string_safe(&value))),
        None => substitute_default(default, ctx, options, functions, depth),
    }
}

/// A scope lookup miss with a `:-` default substitutes the default,
/// re-resolved through the full pipeline so chained defaults work.
/// Past [`MAX_DEFAULT_DEPTH`] the expression counts as a plain miss.
fn substitute_default(
    default: Option<&str>,
    ctx: &Context<'_>,
    options: &Options<'_>,
    functions: &[TemplateFunction],
    depth: u32,
) -> Result<Option<String>, TemplateError> {
    match default {
        Some(text) if depth < MAX_DEFAULT_DEPTH => {
            resolve_pipeline(text, ctx, options, functions, depth + 1).map(Some)
        }
        _ => Ok(None),
    }
}

/// `${secret.NAME}` — the `ctx.get_secret` contract over the secrets
/// record: presence-gated, strict-mode grant matching, lookup miss
/// substitutes `""` and records the name in the missing-secrets
/// ledger (a non-empty default suppresses the miss entirely).
fn resolve_secret(
    name: &str,
    default: Option<&str>,
    ctx: &Context<'_>,
    options: &Options<'_>,
    functions: &[TemplateFunction],
    depth: u32,
) -> Result<Option<String>, TemplateError> {
    if !cap_gate(ctx, "secrets") {
        // Ungated — the placeholder is unresolved; a default still
        // applies, like any other miss.
        return substitute_default(default, ctx, options, functions, depth);
    }
    check_capability("secrets", name, ctx)?;
    let Some(value) = options.scope.secrets.and_then(|secrets| secrets.get(name)) else {
        if let Some(text) = default
            && !text.is_empty()
            && depth < MAX_DEFAULT_DEPTH
        {
            return resolve_pipeline(text, ctx, options, functions, depth + 1).map(Some);
        }
        if ctx.strict {
            ctx.missing_secrets.borrow_mut().push(name.to_string());
        }
        return Ok(Some(String::new()));
    };
    Ok(Some(value.clone()))
}

/// `event.*` — envelope fields, `payload.<path>` into the payload, and
/// `match.<N>`/`shard` wildcard captures from the first `on` pattern
/// matching the bound trigger.
fn lookup_event(segments: &[&str], scope: Option<&EventScope<'_>>) -> Option<Value> {
    let scope = scope?;
    match segments {
        ["type"] => Some(Value::String(scope.event.kind.clone())),
        ["id"] => Some(Value::String(scope.event.id.clone())),
        ["source"] => Some(Value::String(scope.event.source.clone())),
        ["ts"] => Some(Value::from(scope.event.ts)),
        ["payload", rest @ ..] => walk(&scope.event.payload, rest),
        ["match", n] => event_captures(scope)
            .and_then(|caps| caps.get(*n).cloned())
            .map(Value::String),
        ["shard"] => event_captures(scope)
            .and_then(|caps| caps.get("shard").cloned())
            .map(Value::String),
        _ => None,
    }
}

/// Wildcard captures from the first `on` pattern matching the bound
/// trigger (`capture_wildcards`) — `None` when no pattern carries a
/// capture for this event.
fn event_captures(scope: &EventScope<'_>) -> Option<std::collections::HashMap<String, String>> {
    scope
        .patterns
        .iter()
        .find_map(|pattern| peregrin_core::capture_wildcards(pattern, &scope.event.kind))
}

/// `node.*` — `id` (string) and `fires` (count including this fire).
fn lookup_node(segments: &[&str], scope: Option<&crate::scope::NodeScope<'_>>) -> Option<Value> {
    let scope = scope?;
    match segments {
        ["id"] => Some(Value::String(scope.id.to_string())),
        ["fires"] => Some(Value::from(scope.fires)),
        _ => None,
    }
}

/// `params.*` — walk the bound params object. `Resolved` reads the
/// fire's resolved map; `Declared` resolves the referenced key lazily
/// through the full pipeline (memoized, cycle-guarded).
fn lookup_params(
    segments: &[&str],
    ctx: &Context<'_>,
    options: &Options<'_>,
) -> Result<Option<Value>, TemplateError> {
    match &options.scope.params {
        ParamsScope::Resolved(params) => Ok(params
            .get(segments[0])
            .and_then(|v| walk(v, &segments[1..]))),
        ParamsScope::Declared(state) => {
            let Some(node) = options.node else {
                return Ok(None);
            };
            let Some(raw) = node.params.get(segments[0]) else {
                return Ok(None);
            };
            if state.borrow().resolving.contains(segments[0]) {
                // Circular reference — the in-progress key never
                // surfaces; the lookup misses.
                return Ok(None);
            }
            resolve_declared_key(segments[0], raw, state, ctx, options)
                .map(|value| walk(&value, &segments[1..]))
        }
        ParamsScope::Unbound => Ok(None),
    }
}

/// Resolve one declared param key — memoized for the rest of the pass;
/// `resolving` marks the key while its template runs so `${params.k}`
/// self-references cycle out to a miss.
fn resolve_declared_key(
    key: &str,
    raw: &Value,
    state: &std::rc::Rc<std::cell::RefCell<crate::scope::ParamScope>>,
    ctx: &Context<'_>,
    options: &Options<'_>,
) -> Result<Value, TemplateError> {
    if let Some(value) = state.borrow().memo.get(key) {
        return Ok(value.clone());
    }
    state.borrow_mut().resolving.insert(key.to_string());
    let result = resolve_value_opts(raw, ctx, options);
    state.borrow_mut().resolving.remove(key);
    let value = result?;
    state
        .borrow_mut()
        .memo
        .insert(key.to_string(), value.clone());
    Ok(value)
}

/// Walk `segments` into nested objects — a missing key or non-object
/// step is a miss.
fn walk(value: &Value, segments: &[&str]) -> Option<Value> {
    let mut current = value;
    for segment in segments {
        current = current.as_object()?.get(*segment)?;
    }
    Some(current.clone())
}
