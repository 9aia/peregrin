//! peregrin-driver-ops — `driver: ops`, the generic worker-ops driver
//! (P-096). A campaign node names a tamias driver and one of its
//! ops-manifest queries; the driver spawns `tamias serve` (or any
//! ndjson JSON-RPC serve of the same wire), writes ONE
//! `{"op":"driver.query","driver":…,"query":…,"params":…}` line, reads
//! ONE response line, and kills the child — spawn-per-invoke is the
//! v1 contract (pooled serve sessions are a documented follow-up).
//!
//! The wire is the shared `{"op":…}` → `{"ok":true,"result":…}` /
//! `{"ok":false,"error":…}` ndjson JSON-RPC every sibling `serve`
//! speaks (`tamias serve`, `sepia serve`, `myco serve`, `cuora
//! serve`). Domain expertise stays in the domain repos — this driver
//! knows only the wire; `params.serve_argv` (default
//! `["tamias","serve"]`) is the seam for tests and non-tamias serves.
//!
//! `tamias serve`'s `driver.query` op proxies `{"op":"query",…}` to
//! the discovered `tamias-driver-<name>` binary's own `serve` and
//! returns the driver's full `{ok,result|error}` envelope as the op
//! result — a driver-side failure arrives here as the serve's
//! `ok:false`. On `ok:true` the emitted payload carries
//! `result` = the driver's query result (the pavo `QueryResult`
//! `{query, data}` envelope) plus any `params.echo` fields copied
//! from the triggering event's payload.
//!
//! Params (all template-resolved like every driver):
//!
//! - `driver` (string, required) — tamias driver name, e.g. `"lasius"`.
//! - `query` (string, required) — ops-manifest query, e.g.
//!   `"cli.forge.pr.merge"`.
//! - `params` (object, optional) — forwarded as the query params;
//!   `${secret.*}`/`${event.*}`/`${params.*}` references resolve per
//!   the usual template rules before the wire write.
//! - `serve_argv` (array of strings, optional) — the serve command;
//!   default `["tamias","serve"]` (also accepts `["tamias-serve"]`
//!   for hosts where `tamias` itself isn't on PATH).
//! - `timeout` (number, optional) — seconds; default 120, `0` disables
//!   the deadline, capped at 2h like `shell`. On expiry the serve
//!   child is killed and the failure emit fires. Also forwarded to
//!   the serve as `timeout_ms` so a well-behaved serve can answer
//!   before the kill.
//! - `echo` (array of strings, optional) — dotted paths into the
//!   incoming event's `payload` (e.g. `"number"`,
//!   `"pull_request.head.sha"`); each resolved value is written back
//!   into the emitted event's payload at the same path — the
//!   payload-composition feature that replaces file-based state
//!   passing between campaign nodes.
//! - `env` (object, optional) — extra env vars for the serve child
//!   (same contract as `driver: shell`'s `params.env`); the base env
//!   is the sanitized [`peregrin_core::child_env`] allowlist.
//! - `emit` (object, optional) — event-kind overrides: `success`/`ok`/
//!   `true` name the ok event, `failure`/`error`/`false`/`default` the
//!   failure event. Defaults: success → `emits[0]` or `ops.done`;
//!   failure → `emits[1]` or `node.failed` (the `shell` driver's
//!   convention).
//!
//! Failure semantics mirror `driver: shell`: a missing `driver`/`query`
//! param or a template error is a [`DriverError`] (engine retry);
//! spawn/timeout/I/O/`ok:false` outcomes emit the failure event
//! instead — `node.failed` unless `emits[1]`/`emit` says otherwise.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.timeout` default — 120 s, matching `tamias serve`'s own
/// `driver.*` op bound (`DRIVER_OP_TIMEOUT`); `0` disables the
/// deadline (the serve is still killed once the response lands).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
/// Ceiling on a yaml-provided `params.timeout` — unbounded children
/// are the ADR-0005 failure mode (same cap as `driver: shell`).
const MAX_TIMEOUT: Duration = Duration::from_secs(7200);
/// Cap on captured stderr — a chatty serve must not grow the daemon's
/// memory; the tail is kept for error context.
const MAX_CAPTURE: usize = 1024 * 1024;

/// Cancel-poll cadence for the serve wait — the run's
/// `workflow.cancel` flag is checked between recv slices so a
/// superseded run can't sit in an unbounded `timeout: None` wait
/// (I-017).
const POLL: Duration = Duration::from_millis(50);

/// `driver: ops`. `effects` backs the driver-side param re-resolve
/// the TS performs inside `invoke`; `output` holds the event schemas.
pub struct OpsDriver {
    effects: Arc<dyn Effects>,
    output: Vec<EventSchema>,
}

impl Default for OpsDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl OpsDriver {
    /// A driver over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
            output: output_schemas(),
        }
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self {
            effects,
            output: output_schemas(),
        }
    }

    /// Re-resolve `node.params` the way the other drivers do inside
    /// `invoke` (the runner resolves too — the driver-level pass keeps
    /// direct invocations working). `ctx` lends its strict flag,
    /// capability map, and shared missing-secret ledger — this is
    /// where `${secret.*}` references in `params.params` resolve.
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
        let secret_view = |name: &str| ctx.resolve_secret(name);
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
                    secrets: Some(&secret_view),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }
}

/// `input` contract — `driver`/`query` required; the rest optional.
fn input_schema() -> EventSchema {
    EventSchema {
        kind: String::new(),
        fields: Map::from_iter([
            ("driver".to_string(), Value::String("string".to_string())),
            ("query".to_string(), Value::String("string".to_string())),
            ("params".to_string(), Value::String("object?".to_string())),
            (
                "serve_argv".to_string(),
                Value::String("array?".to_string()),
            ),
            ("timeout".to_string(), Value::String("number?".to_string())),
            ("echo".to_string(), Value::String("array?".to_string())),
            ("env".to_string(), Value::String("object?".to_string())),
            ("emit".to_string(), Value::String("object?".to_string())),
        ]),
        schema: Map::new(),
        annotations: Map::new(),
    }
}

/// `output` contracts — the success kind is `emits[0]`-dependent, so
/// both declared shapes are listed generically.
fn output_schemas() -> Vec<EventSchema> {
    let schema = |kind: &str, fields: &[(&str, &str)]| EventSchema {
        kind: kind.to_string(),
        fields: fields
            .iter()
            .map(|(k, t)| ((*k).to_string(), Value::String((*t).to_string())))
            .collect(),
        schema: Map::new(),
        annotations: Map::new(),
    };
    vec![
        schema(
            "ops.done",
            &[("result", "any"), ("driver", "string"), ("query", "string")],
        ),
        schema(
            "node.failed",
            &[
                ("error", "string"),
                ("driver", "string"),
                ("query", "string"),
            ],
        ),
    ]
}

/// `params.serve_argv` → `(program, args)` — default
/// `["tamias","serve"]`. A bare string is tolerated as a single-word
/// argv; anything else is a `DriverError`.
fn serve_argv(params: &Map<String, Value>) -> Result<(String, Vec<String>), DriverError> {
    let argv: Vec<String> = match params.get("serve_argv") {
        None | Some(Value::Null) => vec!["tamias".to_string(), "serve".to_string()],
        Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| DriverError::new("ops node 'params.serve_argv' must be strings"))
            })
            .collect::<Result<_, _>>()?,
        Some(_) => {
            return Err(DriverError::new(
                "ops node 'params.serve_argv' must be an array of strings",
            ));
        }
    };
    let Some((program, args)) = argv.split_first() else {
        return Err(DriverError::new(
            "ops node 'params.serve_argv' must not be empty",
        ));
    };
    if program.is_empty() {
        return Err(DriverError::new(
            "ops node 'params.serve_argv[0]' must not be empty",
        ));
    }
    Ok((program.clone(), args.to_vec()))
}

/// `params.timeout` seconds → [`Duration`]; absent/invalid →
/// [`DEFAULT_TIMEOUT`], `<= 0` → `None` (no deadline), capped at
/// [`MAX_TIMEOUT`].
fn timeout_of(params: &Map<String, Value>) -> Option<Duration> {
    let secs = params
        .get("timeout")
        .and_then(Value::as_f64)
        .filter(|s| s.is_finite());
    match secs {
        None => Some(DEFAULT_TIMEOUT),
        Some(s) if s <= 0.0 => None,
        Some(s) => Some(Duration::from_secs_f64(s).min(MAX_TIMEOUT)),
    }
}

/// The `emit` map lookup — `keys` in order, then `default` is *not*
/// folded in here (callers pick which keys apply per outcome, like
/// the shell driver's rc-keyed map).
fn emit_mapped<'a>(emit: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| emit.get(*key).and_then(Value::as_str))
        .filter(|kind| !kind.is_empty())
}

/// Success event kind — `emit.success`/`emit.ok`/`emit.true`, else
/// `emits[0]`, else `ops.done` (the `shell` driver's `emits[0]`
/// convention).
fn success_kind(node: &Node, params: &Map<String, Value>) -> String {
    if let Some(emit) = params.get("emit").and_then(Value::as_object)
        && let Some(kind) = emit_mapped(emit, &["success", "ok", "true"])
    {
        return kind.to_string();
    }
    node.emits
        .first()
        .cloned()
        .unwrap_or_else(|| "ops.done".to_string())
}

/// Failure event kind — `emit.failure`/`emit.error`/`emit.false`/
/// `emit.default`, else `emits[1]`, else `node.failed` (the shell
/// driver's failure default).
fn failure_kind(node: &Node, params: &Map<String, Value>) -> String {
    if let Some(emit) = params.get("emit").and_then(Value::as_object)
        && let Some(kind) = emit_mapped(emit, &["failure", "error", "false", "default"])
    {
        return kind.to_string();
    }
    node.emits
        .get(1)
        .cloned()
        .unwrap_or_else(|| "node.failed".to_string())
}

/// Dotted-path read into `payload` — `"a.b.c"` walks objects; a
/// missing segment or a non-object intermediate yields `None`.
fn get_dotted<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

/// Dotted-path write into an event payload — creates intermediate
/// objects; a non-object intermediate is replaced by an object (the
/// echo contract is "copy the value here", not merge).
fn set_dotted(payload: &mut Map<String, Value>, path: &str, value: Value) {
    let mut segments = path.split('.').peekable();
    let mut current = payload;
    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            current.insert(segment.to_string(), value);
            return;
        }
        let entry = current
            .entry(segment.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        // Safe: just normalized to an object.
        if let Value::Object(map) = entry {
            current = map;
        } else {
            return;
        }
    }
}

/// Copy each `params.echo` path's value from the first trigger's
/// payload into the emitted payload at the same path. Missing paths
/// are skipped silently — echo is best-effort composition, not a
/// contract check (a later node can fail loudly on the absent field
/// if it cares).
fn apply_echo(payload: &mut Map<String, Value>, echo: &[String], triggers: &[Event]) {
    let Some(trigger) = triggers.first() else {
        return;
    };
    for path in echo {
        if let Some(value) = get_dotted(&trigger.payload, path) {
            set_dotted(payload, path, value.clone());
        }
    }
}

/// `params.echo` → the list of dotted paths (non-strings dropped).
fn echo_paths(params: &Map<String, Value>) -> Vec<String> {
    params
        .get("echo")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .filter(|path| !path.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Drain `pipe` keeping at most `cap` bytes — reads past the cap are
/// discarded so the child never blocks on a full pipe.
fn read_capped(mut pipe: impl Read + Send + 'static, cap: usize) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut kept: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if kept.len() < cap {
                        let room = cap - kept.len();
                        kept.extend_from_slice(&buf[..n.min(room)]);
                    }
                }
            }
        }
        kept
    })
}

/// Spawn `serve_argv`, write ONE request line, read ONE response line,
/// kill the child. Spawn-per-invoke is the v1 contract — pooled serve
/// sessions across nodes are a documented follow-up (the campaign
/// engine is sequential, so a pool would only save spawn latency).
///
/// stdin closes after the write so a well-behaved serve loop answers
/// then exits on EOF; the child is killed once the response line
/// lands (or at the deadline) so a serve that ignores EOF can't leak.
/// `timeout: None` waits indefinitely for the first line.
///
/// Returns the parsed response object; `Err` carries a human reason
/// (spawn failure, deadline, EOF without a line, bad JSON).
fn serve_once(
    program: &str,
    args: &[String],
    env: &Map<String, Value>,
    request: &Value,
    timeout: Option<Duration>,
    stop: &dyn Fn() -> bool,
) -> Result<Value, String> {
    let line =
        serde_json::to_string(request).map_err(|e| format!("encode driver.query request: {e}"))?;
    let mut command = Command::new(program);
    command
        .args(args)
        .env_clear()
        .envs(peregrin_core::child_env())
        .envs(
            env.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = command
        .spawn()
        .map_err(|e| format!("spawn {program}: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        // A dead-on-arrival child makes the write fail — that surfaces
        // as "exited without a response" below; no need to fail early.
        let _ = writeln!(stdin, "{line}");
        // Dropping stdin here closes the pipe: the serve loop answers
        // the pending request then exits on EOF.
    }
    // stderr drains on a thread so a chatty serve can't wedge the
    // read on a full pipe; the tail is kept for error context.
    let stderr_handle = child
        .stderr
        .take()
        .map(|pipe| read_capped(pipe, MAX_CAPTURE));
    // stdout streams lines over a channel — the FIRST line is the
    // response; we don't wait for process exit (a serve loop may keep
    // running after answering).
    let (tx, rx) = mpsc::channel::<String>();
    let stdout_handle = child.stdout.take();
    let stdout_thread = thread::spawn(move || {
        let Some(stdout) = stdout_handle else { return };
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(line.trim_end().to_string()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let outcome = match timeout {
        Some(limit) => {
            let deadline = Instant::now() + limit;
            loop {
                let now = Instant::now();
                if now >= deadline {
                    break Err(format!("{program} serve timed out after {limit:?}"));
                }
                if stop() {
                    break Err(format!("{program} serve cancelled"));
                }
                match rx.recv_timeout((deadline - now).min(POLL)) {
                    Ok(line) => break parse_response(&line),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break Err(format!("{program} serve: exited without a response"));
                    }
                }
            }
        }
        None => loop {
            // `timeout: None` waits indefinitely — but still polls
            // the run's cancel flag so a supersede can't strand the
            // run inside `invoke` (I-017).
            if stop() {
                break Err(format!("{program} serve cancelled"));
            }
            match rx.recv_timeout(POLL) {
                Ok(line) => break parse_response(&line),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(format!("{program} serve: exited without a response"));
                }
            }
        },
    };
    // Reap: kill a still-running child (the response is already in
    // hand, or the deadline hit), then collect stderr for context.
    let _ = child.kill();
    let _ = child.wait();
    let _ = stdout_thread.join();
    let stderr_tail = stderr_handle
        .map(|h| h.join().unwrap_or_default())
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .unwrap_or_default();
    match outcome {
        Ok(value) => Ok(value),
        Err(error) => {
            if stderr_tail.is_empty() {
                Err(error)
            } else {
                Err(format!("{error} — stderr: {stderr_tail}"))
            }
        }
    }
}

/// Parse one response line into the `{"ok":…}` envelope object.
fn parse_response(line: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(line).map_err(|e| format!("bad response line: {e}"))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(format!("malformed response (not an object): {value}"))
    }
}

/// The `result` payload for a successful `driver.query` — the serve
/// returns the driver's `{ok,result}` envelope as the op result, so
/// unwrap one envelope layer when present (`{ok:…, result: QueryResult}`
/// → the `QueryResult`); a bare result passes through verbatim so a
/// serve that doesn't wrap still works.
fn unwrap_result(result: &Value) -> Value {
    if let Some(inner) = result.get("result")
        && result.get("ok").is_some()
    {
        return inner.clone();
    }
    result.clone()
}

/// The error text of an `ok:false` envelope — `error` as a string,
/// else the JSON rendering.
fn error_text(response: &Value) -> String {
    response.get("error").map_or_else(
        || response.to_string(),
        |error| match error {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        },
    )
}

impl Driver for OpsDriver {
    fn name(&self) -> &'static str {
        "ops"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(input_schema())
    }

    fn output(&self) -> &[EventSchema] {
        &self.output
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let required = |key: &str| -> Result<String, DriverError> {
            params
                .get(key)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    DriverError::new(format!("ops node '{}' missing 'params.{key}'", node.id))
                })
        };
        let driver = required("driver")?;
        let query = required("query")?;
        let (program, args) = serve_argv(&params)?;
        let timeout = timeout_of(&params);
        let query_params = match params.get("params") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(_) => {
                return Err(DriverError::new(
                    "ops node 'params.params' must be an object",
                ));
            }
        };
        let env = params
            .get("env")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let echo = echo_paths(&params);

        if ctx.dry_run {
            let mut payload = Map::from_iter([
                ("dry_run".to_string(), Value::Bool(true)),
                ("driver".to_string(), Value::String(driver)),
                ("query".to_string(), Value::String(query)),
            ]);
            apply_echo(&mut payload, &echo, triggers);
            return Ok(vec![Event::new(
                success_kind(node, &params),
                Value::Object(payload),
                &node.id,
            )]);
        }

        // The wire request — `driver.query` proxies
        // `{"op":"query","query":…,"params":…}` to the driver's own
        // serve; `timeout_ms` lets a well-behaved serve answer before
        // our deadline kill (the serve clamps it itself).
        let mut request = Map::from_iter([
            ("op".to_string(), Value::String("driver.query".to_string())),
            ("driver".to_string(), Value::String(driver.clone())),
            ("query".to_string(), Value::String(query.clone())),
            ("params".to_string(), Value::Object(query_params)),
        ]);
        if let Some(limit) = timeout {
            // `timeout` is capped at MAX_TIMEOUT (2h), so the millis
            // fit u64 comfortably; the serve clamps to its own 30min
            // ceiling regardless.
            request.insert(
                "timeout_ms".to_string(),
                Value::from(limit.as_millis() as u64),
            );
        }

        // The run's cancel flag — a supersede mid-query kills the
        // serve child (kill+wait below) instead of waiting out the
        // deadline inside a worktree the dying run tears down (I-017).
        let stop = || ctx.shutdown_requested();
        let response = serve_once(
            &program,
            &args,
            &env,
            &Value::Object(request),
            timeout,
            &stop,
        );

        let mut payload = Map::new();
        payload.insert("driver".to_string(), Value::String(driver.clone()));
        payload.insert("query".to_string(), Value::String(query.clone()));

        let outcome = match response {
            Err(error) => Err(error),
            Ok(envelope) => match envelope.get("ok").and_then(Value::as_bool) {
                Some(true) => Ok(unwrap_result(
                    envelope.get("result").unwrap_or(&Value::Null),
                )),
                Some(false) => Err(error_text(&envelope)),
                None => Err(format!("malformed response (no 'ok' flag): {envelope}")),
            },
        };

        match outcome {
            Ok(result) => {
                payload.insert("result".to_string(), result);
                apply_echo(&mut payload, &echo, triggers);
                Ok(vec![Event::new(
                    success_kind(node, &params),
                    Value::Object(payload),
                    &node.id,
                )])
            }
            Err(error) => {
                payload.insert("error".to_string(), Value::String(error));
                apply_echo(&mut payload, &echo, triggers);
                Ok(vec![Event::new(
                    failure_kind(node, &params),
                    Value::Object(payload),
                    &node.id,
                )])
            }
        }
    }
}
