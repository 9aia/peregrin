//! peregrin-driver-human — `driver: human`, port of `ac-driver-human`.
//!
//! Human-in-the-loop: `params.prompt` (required, `params.prompt_file`
//! prepends file contents) is written to stderr and a line is read
//! from stdin — the TS `readline.question` on `process.stdin`/
//! `process.stderr`. `params.timeout` (seconds, `Number()`-coerced,
//! default 300 — ADR-0005; `0` waits forever) races the answer: a
//! timeout emits the reject arm with `timed_out: true`. `params.expect`
//! pins an exact answer; otherwise any non-empty trimmed response is
//! `ok`. `params.emit_ok`/`params.emit_reject` override the event
//! types, which fall back to `emits[0]`/`{id}.ok` and
//! `emits[1]`/`emits.last`/`{id}.rejected`.
//!
//! The blocking stdin read is the sync equivalent of the TS
//! `AbortController` race: with a deadline the prompt runs on a
//! detached thread and `recv_timeout` bounds it — an unanswered read
//! lingers until stdin closes, matching the abort's leftover readline.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::{Map, Value};

use peregrin_core::{Event, Node, js_string};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.timeout` default — 300 s (ADR-0005).
const DEFAULT_TIMEOUT_SECONDS: f64 = 300.0;

/// The stdin/stderr seam behind `readline.question(prompt)` — a
/// [`Prompter`] writes the question and blocks for the answer line.
/// Tests inject a fake; production uses [`StdinPrompter`].
pub trait Prompter: Send + Sync {
    /// Print `prompt`, return the raw answer line.
    fn question(&self, prompt: &str) -> Result<String, String>;
}

/// The production prompter — prompt verbatim to stderr, one line from
/// stdin (`createInterface({input: process.stdin, output:
/// process.stderr})`).
pub struct StdinPrompter;

impl Prompter for StdinPrompter {
    #[cfg(not(coverage))]
    fn question(&self, prompt: &str) -> Result<String, String> {
        use std::io::Write;
        let mut stderr = std::io::stderr();
        stderr
            .write_all(prompt.as_bytes())
            .and_then(|()| stderr.flush())
            .map_err(|e| e.to_string())?;
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        Ok(line)
    }

    /// Coverage probe — headless stdin can't be exercised on the CI
    /// host, so the real body is compiled out under `cfg(coverage)`
    /// (the launchd/systemd convention).
    #[cfg(coverage)]
    fn question(&self, _prompt: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

/// `driver: human`. `effects` backs the param re-resolve and the
/// `prompt_file` read; `prompter` is the question seam.
pub struct HumanDriver {
    effects: Arc<dyn Effects>,
    prompter: Arc<dyn Prompter>,
}

impl Default for HumanDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanDriver {
    /// A driver over [`SystemEffects`] and [`StdinPrompter`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
            prompter: Arc::new(StdinPrompter),
        }
    }

    /// A driver with a custom prompter (tests, non-tty hosts).
    #[must_use]
    pub fn with_prompter(prompter: Arc<dyn Prompter>) -> Self {
        Self {
            effects: Arc::new(SystemEffects),
            prompter,
        }
    }

    /// Fully custom — effects and prompter.
    #[must_use]
    pub fn with_parts(effects: Arc<dyn Effects>, prompter: Arc<dyn Prompter>) -> Self {
        Self { effects, prompter }
    }

    /// Re-resolve `node.params` the way the TS driver does inside
    /// `invoke` (the runner resolves too — the driver-level pass keeps
    /// direct invocations working). `ctx` lends its strict flag,
    /// capability map, and shared missing-secret ledger.
    fn resolve_params(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Map<String, Value>, DriverError> {
        let tctx = Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &*self.effects,
        };
        let state_view = |key: &str| ctx.state.borrow().get(key);
        let resolved = resolve_value_opts(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &[],
                scope: Scope {
                    event: triggers.first().map(|event| EventScope {
                        event,
                        patterns: &node.on,
                    }),
                    config: Some(&ctx.config),
                    params: ParamsScope::Resolved(&ctx.params),
                    state: Some(&state_view),
                    node: Some(NodeScope {
                        id: &node.id,
                        fires: ctx.fires,
                    }),
                    secrets: Some(&ctx.secrets),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }
}

/// `Number(v)` — strings parse (empty/non-numeric → `NaN`), booleans
/// are 0/1, arrays join-then-parse (`[5]` → 5, `[1,2]` → `NaN`),
/// objects → `NaN`. `null` never reaches here — `?? 300` handled it.
fn js_number(value: &Value) -> f64 {
    match value {
        // `null`/`undefined` never reach here — `?? 300` caught them;
        // objects are `NaN` in JS.
        Value::Null | Value::Object(_) => f64::NAN,
        Value::Bool(b) => f64::from(*b as u8),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
        Value::Array(items) => js_string(&Value::Array(items.clone()))
            .trim()
            .parse::<f64>()
            .unwrap_or(f64::NAN),
    }
}

/// `question` raced against the timeout (`Effect.race(question,
/// timeout)`): the prompt runs on a detached thread; `timeout` of
/// `None` (0 or `NaN` seconds) is `Effect.never` — block directly.
/// Returns `(raw_answer, timed_out)`; a prompter failure is the
/// `question` effect's `DriverError`.
fn ask(
    prompter: &Arc<dyn Prompter>,
    prompt: &str,
    timeout: Option<Duration>,
) -> Result<(String, bool), DriverError> {
    let Some(deadline) = timeout else {
        return prompter
            .question(prompt)
            .map(|answer| (answer, false))
            .map_err(DriverError::new);
    };
    let prompter = Arc::clone(prompter);
    let prompt = prompt.to_string();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(prompter.question(&prompt));
    });
    match rx.recv_timeout(deadline) {
        Ok(Ok(answer)) => Ok((answer, false)),
        Ok(Err(error)) => Err(DriverError::new(error)),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok((String::new(), true)),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(DriverError::new("prompter thread died before answering"))
        }
    }
}

impl Driver for HumanDriver {
    fn name(&self) -> &'static str {
        "human"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let prompt = params
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if prompt.is_empty() {
            return Err(DriverError::new(format!(
                "human node '{}' missing 'params.prompt'",
                node.id
            )));
        }

        let mut final_prompt = prompt.to_string();
        if let Some(prompt_file) = params
            .get("prompt_file")
            .and_then(Value::as_str)
            .filter(|file| !file.is_empty())
        {
            let path = if Path::new(prompt_file).is_absolute() {
                PathBuf::from(prompt_file)
            } else {
                self.effects.cwd().join(prompt_file)
            };
            let bytes = self.effects.read_file(&path).map_err(DriverError::new)?;
            let contents = String::from_utf8_lossy(&bytes);
            if !contents.is_empty() {
                final_prompt = format!("{contents}\n{prompt}");
            }
        }

        let expect = params.get("expect").and_then(Value::as_str);
        // `Number(rawTimeout ?? 300)` — null/absent fall to the default.
        let timeout_seconds = match params.get("timeout") {
            None | Some(Value::Null) => DEFAULT_TIMEOUT_SECONDS,
            Some(value) => js_number(value),
        };
        let timeout = (timeout_seconds > 0.0 && timeout_seconds.is_finite())
            .then(|| Duration::from_secs_f64(timeout_seconds));

        let emit_ok = params.get("emit_ok").and_then(Value::as_str);
        let emit_reject = params.get("emit_reject").and_then(Value::as_str);

        let ok_kind = || {
            emit_ok
                .map(str::to_string)
                .or_else(|| node.emits.first().cloned())
                .unwrap_or_else(|| format!("{}.ok", node.id))
        };

        if ctx.dry_run {
            return Ok(vec![Event::new(
                ok_kind(),
                serde_json::json!({
                    "response": "(dry-run)",
                    "ok": true,
                    "timed_out": false,
                }),
                &node.id,
            )]);
        }

        let (raw, timed_out) = ask(&self.prompter, &final_prompt, timeout)?;
        let response = raw.trim();

        let ok = if timed_out {
            false
        } else if let Some(expect) = expect {
            response == expect
        } else {
            !response.is_empty()
        };

        let kind = if ok {
            ok_kind()
        } else {
            emit_reject
                .map(str::to_string)
                .or_else(|| node.emits.get(1).cloned())
                .or_else(|| node.emits.last().cloned())
                .unwrap_or_else(|| format!("{}.rejected", node.id))
        };

        Ok(vec![Event::new(
            kind,
            serde_json::json!({
                "response": response,
                "ok": ok,
                "timed_out": timed_out,
            }),
            &node.id,
        )])
    }
}
