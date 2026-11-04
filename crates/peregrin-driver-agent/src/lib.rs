//! peregrin-driver-agent — `driver: agent`, port of `ac-driver-agent`.
//!
//! A node either calls a local LiteLLM chat-completions endpoint
//! (`params.model` set — the TS `handl-plugin-litellm` `complete`
//! path) or spawns an agent binary. Runners:
//!
//! - `devin` (default) — `devin -p [--prompt-file F | PROMPT]
//!   [--permission-mode M] --respect-workspace-trust <trust>`.
//! - `custom` — `params.binary` + `params.flags` (+ `params.extra_args`),
//!   the prompt on stdin.
//!
//! `params.env` merges over the process env; `params.secrets` injects
//! named secrets present in `ctx.secrets` (presence check only — no
//! miss audit). `params.emit` maps `rc` → event type with a `default`
//! arm; otherwise `rc == 0` emits `emits[0]`/`{id}.done` and anything
//! else `emits[1]`/`emits[0]`/`{id}.done`. Spawn failures and the
//! `params.timeout` (default 600 s, `0` = no limit) emit `node.failed`
//! with `rc: -1`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_core::{Event, Node, js_string};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.timeout` default — 600 s (ADR-0005 keeps it).
const DEFAULT_TIMEOUT_SECONDS: f64 = 600.0;
/// The LiteLLM proxy default (`handl-plugin-litellm`).
const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4000/v1/chat/completions";
/// `chatCompletion`'s timeout default — 120 s.
const LITELLM_TIMEOUT: Duration = Duration::from_secs(120);
/// Poll cadence while waiting on the child.
const POLL: Duration = Duration::from_millis(5);
/// Cap on captured stdout/stderr — agents can run for `params.timeout`
/// seconds emitting output the whole time; unbounded capture is a
/// memory DoS. Past the cap we keep draining (so the child never
/// blocks on a full pipe) and truncate.
const MAX_CAPTURE: usize = 10 * 1024 * 1024;

/// `driver: agent`. `effects` backs the driver-side param re-resolve
/// the TS performs inside `invoke`, plus the `process.cwd()`/
/// `readFileSync` calls that resolve `cwd`/`prompt_file`.
pub struct AgentDriver {
    effects: Arc<dyn Effects>,
}

impl Default for AgentDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDriver {
    /// A driver over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
        }
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self { effects }
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

/// The TS `LiteLLMRequest` — `complete`'s inputs.
struct LiteRequest<'a> {
    model: &'a str,
    provider: Option<&'a str>,
    prompt: &'a str,
    endpoint: Option<&'a str>,
    api_key: Option<&'a str>,
    system: Option<&'a str>,
}

/// `complete`/`chatCompletion` — POST `{model, messages}` to the
/// chat-completions endpoint and extract `choices[0].message.content`.
/// `provider` prefixes the model as `provider/model`.
fn litellm_complete(request: &LiteRequest<'_>) -> Result<String, DriverError> {
    let endpoint = request.endpoint.unwrap_or(DEFAULT_ENDPOINT);
    let model = request.provider.map_or_else(
        || request.model.to_string(),
        |provider| format!("{provider}/{}", request.model),
    );
    let mut messages = Vec::new();
    if let Some(system) = request.system {
        messages.push(serde_json::json!({ "role": "system", "content": system }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": request.prompt }));

    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(LITELLM_TIMEOUT))
            .build(),
    );
    let mut builder = ureq::http::Request::builder()
        .method("POST")
        .uri(endpoint)
        .header("Content-Type", "application/json");
    if let Some(api_key) = request.api_key {
        builder = builder.header("Authorization", format!("Bearer {api_key}"));
    }
    let body = serde_json::json!({ "model": model, "messages": messages }).to_string();
    let request_error = |error: ureq::http::Error| {
        DriverError::new(format!(
            "litellm request failed for model '{model}': {error}"
        ))
    };
    let mut response = agent
        .run(builder.body(body).map_err(request_error)?)
        .map_err(|error| {
            DriverError::new(format!(
                "litellm request failed for model '{model}': {error}"
            ))
        })?;
    if !response.status().is_success() {
        return Err(DriverError::new(format!(
            "litellm request failed with status {}",
            response.status().as_u16()
        )));
    }
    let raw: Value = response
        .body_mut()
        .read_json()
        .map_err(|error| DriverError::new(format!("failed to parse litellm response: {error}")))?;
    raw.get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| DriverError::new("litellm response missing choices[0].message.content"))
}

/// Sanitized base env + `params.env` + `params.secrets` entries — each
/// a `key in ctx.secrets` presence check (no miss audit, unlike
/// `ctx.secrets[key]`). Non-string env values coerce via `String(v)`.
///
/// The base is [`peregrin_core::child_env`], not the whole process
/// env: agent binaries must only see the secrets a campaign declares
/// (`params.secrets`) plus its explicit `params.env` — never ambient
/// daemon secrets that happened to be materialized in the parent env.
fn build_env(params: &Map<String, Value>, ctx: &ExecutionContext) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = peregrin_core::child_env().into_iter().collect();
    if let Some(resolved) = params.get("env").and_then(Value::as_object) {
        for (key, value) in resolved {
            env.insert(key.clone(), js_string(value));
        }
    }
    if let Some(requested) = params.get("secrets").and_then(Value::as_array) {
        for key in requested.iter().filter_map(Value::as_str) {
            if let Some(value) = ctx.secrets.get(key) {
                env.insert(key.to_string(), value.clone());
            }
        }
    }
    env
}

/// Drain `pipe` to EOF keeping at most [`MAX_CAPTURE`] bytes — reads
/// past the cap are discarded (not left in the pipe) so the child
/// always runs to completion and never deadlocks or dies on EPIPE.
fn read_pipe(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut kept: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if kept.len() < MAX_CAPTURE {
                        let room = MAX_CAPTURE - kept.len();
                        kept.extend_from_slice(&buf[..n.min(room)]);
                    }
                }
            }
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

/// `spawnSync(binary, argv, {cwd, env, input, timeout})` — pipes drain
/// on reader threads and stdin gets its own writer so a child that
/// never reads can't deadlock the poll. `timeout` kills the child and
/// reports like Node's `ETIMEDOUT` (`result.error` → `node.failed`);
/// `None` means no limit. Returns `(rc, stdout, stderr)` where `rc`
/// is `status ?? -1`; `Err` mirrors `result.error`.
fn spawn_runner(
    binary: &str,
    argv: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    input: Option<&str>,
    timeout: Option<Duration>,
) -> Result<(i64, String, String), String> {
    let mut command = Command::new(binary);
    command
        .args(argv)
        .current_dir(cwd)
        // `envs` merges over the inherited env — clear first or the
        // daemon's ambient env (secrets included) leaks into the child.
        .env_clear()
        .envs(env)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = command.spawn().map_err(|e| e.to_string())?;
    let stdin_handle = child.stdin.take().map(|mut pipe| {
        let input = input.unwrap_or_default().to_string();
        thread::spawn(move || {
            let _ = pipe.write_all(input.as_bytes());
        })
    });
    let stdout_handle = child.stdout.take().map(read_pipe);
    let stderr_handle = child.stderr.take().map(read_pipe);
    let deadline = timeout.map(|t| Instant::now() + t);
    let status = loop {
        // `try_wait` errors fold into "still running" — the deadline
        // (or the child finishing) always breaks the loop.
        if let Some(status) = child.try_wait().ok().flatten() {
            break Ok(Some(status));
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            let _ = child.kill();
            let _ = child.wait();
            break Err(format!("spawnSync {binary} ETIMEDOUT"));
        }
        thread::sleep(POLL);
    };
    if let Some(handle) = stdin_handle {
        let _ = handle.join();
    }
    let stdout = stdout_handle.map_or_else(String::new, |h| h.join().unwrap_or_default());
    let stderr = stderr_handle.map_or_else(String::new, |h| h.join().unwrap_or_default());
    let status = status?;
    // `result.status ?? -1` — a killed child has no exit code.
    let rc = status.and_then(|s| s.code()).map_or(-1, i64::from);
    Ok((rc, stdout, stderr))
}

/// `getEventType(rc)` — the `emit` map's `rc`-keyed arm, then
/// `default`; otherwise `rc == 0` → `emits[0]`/`{id}.done`, nonzero →
/// `emits[1]`/`emits[0]`/`{id}.done`.
fn event_type(node: &Node, params: &Map<String, Value>, rc: i64) -> String {
    if let Some(emit) = params.get("emit").and_then(Value::as_object)
        && let Some(mapped) = emit
            .get(&rc.to_string())
            .or_else(|| emit.get("default"))
            .and_then(Value::as_str)
    {
        return mapped.to_string();
    }
    if rc == 0 {
        return node
            .emits
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{}.done", node.id));
    }
    node.emits
        .get(1)
        .or_else(|| node.emits.first())
        .cloned()
        .unwrap_or_else(|| format!("{}.done", node.id))
}

/// A non-empty string param, else `None` (the TS
/// `typeof x === "string" && x.length > 0 ? x : undefined` pattern).
fn non_empty_str<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

impl Driver for AgentDriver {
    fn name(&self) -> &'static str {
        "agent"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let runner = params
            .get("runner")
            .and_then(Value::as_str)
            .unwrap_or("devin")
            .to_string();
        let cwd = params
            .get("cwd")
            .and_then(Value::as_str)
            .map_or_else(|| self.effects.cwd(), PathBuf::from);
        // `spawnSync`'s `timeout` — 0/undefined means no limit.
        let timeout = params
            .get("timeout")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS);
        let timeout =
            (timeout > 0.0 && timeout.is_finite()).then(|| Duration::from_secs_f64(timeout));

        let env = build_env(&params, ctx);

        let prompt_text = if let Some(prompt) = non_empty_str(&params, "prompt") {
            prompt.to_string()
        } else if let Some(prompt_file) = non_empty_str(&params, "prompt_file") {
            let path = if Path::new(prompt_file).is_absolute() {
                PathBuf::from(prompt_file)
            } else {
                self.effects.cwd().join(prompt_file)
            };
            let bytes = self.effects.read_file(&path).map_err(|error| {
                DriverError::new(format!(
                    "agent node '{}' failed to read prompt_file: {error}",
                    node.id
                ))
            })?;
            String::from_utf8_lossy(&bytes).into_owned()
        } else {
            return Err(DriverError::new(format!(
                "agent node '{}' missing 'params.prompt' or 'params.prompt_file'",
                node.id
            )));
        };

        // LiteLLM path — `params.model` routes to chat completions.
        if let Some(model) = non_empty_str(&params, "model") {
            if ctx.dry_run {
                let mut payload = Map::from_iter([
                    ("content".to_string(), Value::String(String::new())),
                    ("model".to_string(), Value::String(model.to_string())),
                ]);
                if let Some(provider) = params.get("provider") {
                    payload.insert("provider".to_string(), provider.clone());
                }
                payload.insert("dry_run".to_string(), Value::Bool(true));
                return Ok(vec![Event::new(
                    "agent.litellm",
                    Value::Object(payload),
                    &node.id,
                )]);
            }
            let content = litellm_complete(&LiteRequest {
                model,
                provider: non_empty_str(&params, "provider"),
                prompt: &prompt_text,
                endpoint: non_empty_str(&params, "endpoint"),
                api_key: non_empty_str(&params, "api_key")
                    .map(str::to_string)
                    .or_else(|| env.get("LITELLM_API_KEY").cloned())
                    .as_deref(),
                system: non_empty_str(&params, "system"),
            })?;
            let mut payload = Map::from_iter([
                ("content".to_string(), Value::String(content)),
                ("model".to_string(), Value::String(model.to_string())),
            ]);
            if let Some(provider) = params.get("provider") {
                payload.insert("provider".to_string(), provider.clone());
            }
            return Ok(vec![Event::new(
                "agent.litellm",
                Value::Object(payload),
                &node.id,
            )]);
        }

        let permission_mode = params.get("permission_mode").and_then(Value::as_str);
        let trust = params.get("trust") == Some(&Value::Bool(true));

        let (binary, argv, input): (String, Vec<String>, Option<String>) = match runner.as_str() {
            "devin" => {
                let mut argv = vec!["-p".to_string()];
                if let Some(prompt_file) = non_empty_str(&params, "prompt_file") {
                    argv.push("--prompt-file".to_string());
                    argv.push(prompt_file.to_string());
                } else {
                    argv.push(prompt_text.clone());
                }
                if let Some(mode) = permission_mode {
                    argv.push("--permission-mode".to_string());
                    argv.push(mode.to_string());
                }
                argv.push("--respect-workspace-trust".to_string());
                argv.push(if trust { "true" } else { "false" }.to_string());
                ("devin".to_string(), argv, None)
            }
            "custom" => {
                let binary = non_empty_str(&params, "binary").ok_or_else(|| {
                    DriverError::new(format!(
                        "agent node '{}' custom runner missing 'params.binary'",
                        node.id
                    ))
                })?;
                let flags = params
                    .get("flags")
                    .and_then(Value::as_array)
                    .filter(|items| items.iter().all(Value::is_string))
                    .ok_or_else(|| {
                        DriverError::new(format!(
                            "agent node '{}' custom runner 'params.flags' must be an array of strings",
                            node.id
                        ))
                    })?;
                let mut argv: Vec<String> = flags
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect();
                if let Some(extra) = params.get("extra_args") {
                    let extra = extra
                        .as_array()
                        .filter(|items| items.iter().all(Value::is_string))
                        .ok_or_else(|| {
                            DriverError::new(format!(
                                "agent node '{}' custom runner 'params.extra_args' must be an array of strings",
                                node.id
                            ))
                        })?;
                    argv.extend(
                        extra
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_string)),
                    );
                }
                let input = (!prompt_text.is_empty()).then(|| prompt_text.clone());
                (binary.to_string(), argv, input)
            }
            _ => {
                return Err(DriverError::new(format!("unknown agent runner '{runner}'")));
            }
        };

        if ctx.dry_run {
            return Ok(vec![Event::new(
                event_type(node, &params, 0),
                serde_json::json!({
                    "cmd": argv.join(" "),
                    "dry_run": true,
                    "rc": 0,
                    "stdout": "",
                    "stderr": "",
                    "runner": runner,
                }),
                &node.id,
            )]);
        }

        let (rc, stdout, stderr) =
            match spawn_runner(&binary, &argv, &cwd, &env, input.as_deref(), timeout) {
                Err(error) => {
                    return Ok(vec![Event::new(
                        "node.failed",
                        serde_json::json!({ "error": error, "rc": -1, "runner": runner }),
                        &node.id,
                    )]);
                }
                Ok(result) => result,
            };

        Ok(vec![Event::new(
            event_type(node, &params, rc),
            serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "rc": rc,
                "runner": runner,
            }),
            &node.id,
        )])
    }
}

/// `mock-agent` — a scripted stand-in for [`AgentDriver`] used by
/// orchestration tests and dry CI runs. Same `prompt`/`prompt_file`
/// surface (templating resolves for real), but no agent CLI is
/// spawned: `params.response` becomes the canned `agent.done`
/// payload, `params.fail` forces `node.failed`, `params.delay_ms`
/// sleeps first (timing-sensitive graph tests).
pub struct MockAgentDriver {
    agent: AgentDriver,
}

impl Default for MockAgentDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAgentDriver {
    /// A mock over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: AgentDriver::new(),
        }
    }

    /// A mock over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self {
            agent: AgentDriver::with_effects(effects),
        }
    }
}

impl Driver for MockAgentDriver {
    fn name(&self) -> &'static str {
        "mock-agent"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.agent.resolve_params(node, triggers, ctx)?;
        let prompt = if let Some(prompt) = non_empty_str(&params, "prompt") {
            prompt.to_string()
        } else if let Some(prompt_file) = non_empty_str(&params, "prompt_file") {
            let path = PathBuf::from(prompt_file);
            let path = if path.is_absolute() {
                path
            } else {
                self.agent.effects.cwd().join(path)
            };
            let bytes = self.agent.effects.read_file(&path).map_err(|error| {
                DriverError::new(format!(
                    "mock-agent node '{}' failed to read prompt_file: {error}",
                    node.id
                ))
            })?;
            String::from_utf8_lossy(&bytes).into_owned()
        } else {
            String::new()
        };

        if let Some(delay_ms) = params.get("delay_ms").and_then(Value::as_f64) {
            if delay_ms > 0.0 && delay_ms.is_finite() {
                std::thread::sleep(Duration::from_secs_f64(delay_ms / 1000.0));
            }
        }

        if params.get("fail").and_then(Value::as_bool).unwrap_or(false) {
            let error = params
                .get("fail_message")
                .and_then(Value::as_str)
                .unwrap_or("mock-agent scripted failure");
            return Ok(vec![Event::new(
                "node.failed",
                serde_json::json!({ "error": error, "rc": 1, "runner": "mock" }),
                &node.id,
            )]);
        }

        let response = params
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(vec![Event::new(
            "agent.done",
            serde_json::json!({
                "stdout": response,
                "stderr": "",
                "rc": 0,
                "runner": "mock",
                "prompt": prompt,
            }),
            &node.id,
        )])
    }
}
