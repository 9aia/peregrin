//! The resolver — port of `engine.ts`.
//!
//! Render pass: `${...}` placeholders split on the *first* `}` (the TS
//! regex `/\$\{([^}]+)\}/g` — no nesting). Strict-mode audit: a second
//! scan over the rendered text that *does* count nested braces
//! (`findMatchingBrace`) and fails on whatever a later pass or shell
//! would have seen.

use serde_json::{Map, Value};

use crate::context::{Context, Options};
use crate::error::TemplateError;
use crate::function::{TemplateFunction, builtins};
use crate::js::to_string_safe;
use crate::parse::parse_call;

/// `event`/`config`/`secret`/`node`/`params`/`state` lookups are
/// deferred to the scope-resolution pass — and so are `${file.x}`-style
/// dotted forms under the I/O builtin names.
const KNOWN_PREFIXES: &[&str] = &[
    "event", "config", "secret", "node", "params", "state", "file", "shell", "env", "random",
    "uuid", "now",
];

/// `resolveTemplate` — render `${...}` placeholders, preserving anything
/// unrecognized; in strict mode fail on what remains unresolved.
///
/// The TS signature took an unused `triggers` parameter; it is dropped
/// here. `options` carries the optional `node` and custom functions.
pub fn resolve_template(template: &str, ctx: &Context<'_>) -> Result<String, TemplateError> {
    resolve_template_opts(template, ctx, &Options::default())
}

/// `resolveTemplate` with `node` / `customFunctions`.
pub fn resolve_template_opts(
    template: &str,
    ctx: &Context<'_>,
    options: &Options<'_>,
) -> Result<String, TemplateError> {
    let pre = resolve_defaults(template);
    let functions = all_functions(options.functions);
    let rendered = render_template(&pre, ctx, options.node, &functions)?;
    if ctx.strict {
        let unresolved = find_all_placeholders(&rendered, ctx, &functions);
        if !unresolved.is_empty() {
            return Err(TemplateError::new(format!(
                "unresolved placeholders: {}",
                unresolved.join(", ")
            )));
        }
        if !ctx.missing_secrets.is_empty() {
            return Err(TemplateError::new(format!(
                "missing secrets: {}",
                ctx.missing_secrets.join(", ")
            )));
        }
    }
    Ok(rendered)
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

/// `isJinjaExpr` — a placeholder counts as deferred-to-scope when it's a
/// bare prefix or `prefix.`/`prefix(` path with no `|` filter.
fn is_jinja_expr(content: &str) -> bool {
    let stripped = content.trim();
    if stripped.contains('|') {
        return false;
    }
    KNOWN_PREFIXES.iter().any(|prefix| {
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

/// `isUnresolvable` — after the render pass, would this placeholder have
/// resolved? Calls need a granted known function; anything else needs to
/// look like a deferred scope expression.
fn is_unresolvable(content: &str, ctx: &Context<'_>, functions: &[TemplateFunction]) -> bool {
    if let Some(call) = parse_call(content) {
        let gated = functions.iter().any(|f| f.name == call.name) && cap_gate(ctx, &call.name);
        return !gated;
    }
    !is_jinja_expr(content)
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
