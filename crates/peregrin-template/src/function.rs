//! Template functions — the TS `TemplateFunction` record plus the
//! builtin set (`env`, `file`, `now`, `random`, `shell`, `uuid`).

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use crate::capability::check_capability;
use crate::context::Context;
use crate::error::TemplateError;
use crate::js::js_number_value;

type Call = dyn Fn(
        &[Value],
        &Context<'_>,
        Option<&peregrin_core::Node>,
        Option<&str>,
    ) -> Result<Value, TemplateError>
    + Send
    + Sync;

/// A named `${name(args)}` callable — builtins plus host-registered
/// custom functions (the TS `customFunctions` map).
#[derive(Clone)]
pub struct TemplateFunction {
    /// Name as it appears in `${name(...)}`.
    pub name: String,
    /// Same inputs → same output; marks non-replayable nodes when false
    /// (ADR-0023 — consumed by the runner, not the engine).
    pub deterministic: bool,
    /// Whether strict-mode capability checking applies to its arguments.
    pub requires_capability: bool,
    call: Arc<Call>,
}

impl TemplateFunction {
    /// Wrap a callable. Signature mirrors the TS `call(args, ctx, node,
    /// method)`; `method` is the `.method` selector (`${now.unix()}`).
    pub fn new<F>(name: &str, deterministic: bool, requires_capability: bool, call: F) -> Self
    where
        F: Fn(
                &[Value],
                &Context<'_>,
                Option<&peregrin_core::Node>,
                Option<&str>,
            ) -> Result<Value, TemplateError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            name: name.to_string(),
            deterministic,
            requires_capability,
            call: Arc::new(call),
        }
    }

    /// Invoke the function.
    pub(crate) fn call(
        &self,
        args: &[Value],
        ctx: &Context<'_>,
        node: Option<&peregrin_core::Node>,
        method: Option<&str>,
    ) -> Result<Value, TemplateError> {
        (self.call)(args, ctx, node, method)
    }
}

/// The builtin set — the TS `templateFunctions` record.
pub fn builtins() -> Vec<TemplateFunction> {
    vec![
        TemplateFunction::new("env", true, true, |args, ctx, _node, _method| {
            env_call(args, ctx)
        }),
        TemplateFunction::new("file", true, true, |args, ctx, _node, _method| {
            file_call(args, ctx)
        }),
        TemplateFunction::new("now", false, false, |_args, ctx, _node, method| {
            now_call(ctx, method)
        }),
        TemplateFunction::new("random", false, false, |args, ctx, _node, _method| {
            Ok(random_call(args, ctx))
        }),
        TemplateFunction::new("shell", true, true, |args, ctx, _node, _method| {
            shell_call(args, ctx)
        }),
        TemplateFunction::new("uuid", false, false, |_args, ctx, _node, _method| {
            Ok(uuid_call(ctx))
        }),
    ]
}

/// `${env("NAME")}` / `${env("NAME", "default")}`.
fn env_call(args: &[Value], ctx: &Context<'_>) -> Result<Value, TemplateError> {
    let Some(name) = args.first().and_then(Value::as_str) else {
        return Err(TemplateError::new("env() requires a string name"));
    };
    check_capability("env", name, ctx)?;
    match ctx.effects.env(name) {
        Some(value) => Ok(Value::String(value)),
        // `args.length > 1 ? args[1] : ""` — the default is returned
        // uncoerced (toStringSafe handles numbers later).
        None => Ok(args
            .get(1)
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()))),
    }
}

/// `${file("path")}` — relative paths join `cwd` and normalize like
/// `path.join` before the capability check and the read.
fn file_call(args: &[Value], ctx: &Context<'_>) -> Result<Value, TemplateError> {
    let Some(raw) = args.first().and_then(Value::as_str) else {
        return Err(TemplateError::new("file() requires a string path"));
    };
    let raw_path = Path::new(raw);
    let resolved = if raw_path.is_absolute() {
        normalize(raw_path)
    } else {
        normalize(&ctx.effects.cwd().join(raw_path))
    };
    check_capability("file", &resolved.to_string_lossy(), ctx)?;
    let bytes = ctx
        .effects
        .read_file(&resolved)
        .map_err(|e| TemplateError::new(format!("file() failed: {e}")))?;
    // TextDecoder with fatal: false — lossy decode.
    Ok(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
}

/// `path.join` normalization — collapse `.`/`..` lexically so the
/// capability check sees the same string TS produced.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `${now()}` / `${now.iso()}` / `${now.unix()}` — default method `iso`.
fn now_call(ctx: &Context<'_>, method: Option<&str>) -> Result<Value, TemplateError> {
    match method.unwrap_or("iso") {
        "unix" => Ok(js_number_value(ctx.effects.now_unix() as f64)),
        "iso" => Ok(Value::String(ctx.effects.now_iso())),
        other => Err(TemplateError::new(format!(
            "now.{other}() is not supported"
        ))),
    }
}

/// `${random()}` / `${random(min, max)}` — integer range when both
/// bounds are integral and `max >= min` (`Number.isInteger`), else float.
/// Never fails (the TS `Effect.succeed`).
fn random_call(args: &[Value], ctx: &Context<'_>) -> Value {
    let min = args.first().and_then(Value::as_f64).unwrap_or(0.0);
    let max = args.get(1).and_then(Value::as_f64).unwrap_or(1.0);
    let r = ctx.effects.random();
    if min.is_finite() && max.is_finite() && min.fract() == 0.0 && max.fract() == 0.0 && max >= min
    {
        js_number_value((min + r * (max - min + 1.0)).floor())
    } else {
        js_number_value(min + r * (max - min))
    }
}

/// `${shell("cmd")}` — capability-checked, captures trimmed stdout.
fn shell_call(args: &[Value], ctx: &Context<'_>) -> Result<Value, TemplateError> {
    let Some(raw) = args.first().and_then(Value::as_str) else {
        return Err(TemplateError::new("shell() requires a string command"));
    };
    check_capability("shell", raw, ctx)?;
    ctx.effects
        .shell(raw)
        .map(Value::String)
        .map_err(|e| TemplateError::new(format!("shell() failed: {e}")))
}

/// `${uuid()}` — never fails.
fn uuid_call(ctx: &Context<'_>) -> Value {
    Value::String(ctx.effects.uuid())
}
