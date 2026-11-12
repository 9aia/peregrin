//! peregrin-driver-shell — `driver: shell`, port of `ac-driver-shell`.
//! `params.cmd` runs under the system shell (`spawnSync(cmd,
//! {shell: true, timeout: 30000})` in the TS — 30 s hardcoded).
//! `params.emit` maps `rc` → event type; otherwise `rc == 0` emits
//! `cmd.done` and anything else `node.failed`. `params.cwd` (a
//! Rust-side addition) sets the child's working directory — wrappers
//! like `driver: git` use it to run the inner shell inside a
//! materialized worktree.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node, js_truthy};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `spawnSync`'s default timeout — 30 s (ADR-0005 default);
/// `params.timeout` (seconds) overrides, capped at 2 hours — agent
/// sessions legitimately run tens of minutes (pr-heal campaigns).
const TIMEOUT: Duration = Duration::from_secs(30);
/// Ceiling on a yaml-provided `params.timeout` — unbounded children
/// are the ADR-0005 failure mode.
const MAX_TIMEOUT: Duration = Duration::from_secs(7200);
/// Poll cadence while waiting on the child.
const POLL: Duration = Duration::from_millis(5);
/// Cap on captured stdout/stderr — a chatty or hostile child must not
/// grow the daemon's memory without bound; past the cap we keep
/// draining (so the child never blocks on a full pipe) and truncate.
const MAX_CAPTURE: usize = 10 * 1024 * 1024;

/// `driver: shell`. `effects` backs the driver-side param re-resolve
/// the TS performs inside `invoke`; `output` holds the TS `output`
/// schemas once (they're statics there).
pub struct ShellDriver {
    effects: Arc<dyn Effects>,
    output: Vec<EventSchema>,
}

impl Default for ShellDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellDriver {
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

/// `input` contract — `cmd` required, `timeout`/`emit`/`env`/`cwd`
/// optional (the TS `input` EventSchema; `cwd` is a Rust-side
/// addition so wrappers like `driver: git` can place the child).
fn input_schema() -> EventSchema {
    EventSchema {
        kind: String::new(),
        fields: Map::from_iter([
            ("cmd".to_string(), Value::String("string".to_string())),
            ("timeout".to_string(), Value::String("integer?".to_string())),
            ("emit".to_string(), Value::String("object?".to_string())),
            ("env".to_string(), Value::String("object?".to_string())),
            ("cwd".to_string(), Value::String("string?".to_string())),
        ]),
        schema: Map::new(),
        annotations: Map::new(),
    }
}

/// `output` contracts — `cmd.done` and `node.failed`.
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
        schema("cmd.done", &[("stdout", "string"), ("rc", "integer")]),
        schema(
            "node.failed",
            &[
                ("error", "string"),
                ("rc", "integer"),
                ("stdout", "string?"),
                ("stderr", "string?"),
            ],
        ),
    ]
}

/// `systemd-run` availability — probed once per process. `isolate`
/// params fail honestly when the user manager can't scope the
/// command rather than silently running unconfined.
fn systemd_run() -> Option<&'static str> {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let ok = *AVAILABLE.get_or_init(|| {
        std::env::var_os("PATH").is_some_and(|path| {
            std::env::split_paths(&path).any(|dir| dir.join("systemd-run").is_file())
        })
    });
    ok.then_some("systemd-run")
}

/// Build the spawn `Command` — plain shell, or `systemd-run --user
/// --scope` wrapping it when `isolate`/`memory_max` ask for cgroup
/// confinement (PL-007 C4 job isolation). `memory_max` implies
/// isolation: a cap without a scope is meaningless. Non-unix or a
/// missing `systemd-run` surfaces as `Err` — never a silent
/// unconfined run.
fn spawn_command(cmd: &str, isolate: bool, memory_max: Option<&str>) -> Result<Command, String> {
    #[cfg(unix)]
    {
        let confine = isolate || memory_max.is_some();
        if !confine {
            return Ok(shell_command(cmd));
        }
        let Some(bin) = systemd_run() else {
            return Err(
                "isolate/memory_max requested but `systemd-run` not found on PATH \
                 (needs systemd --user)"
                    .to_string(),
            );
        };
        let mut command = Command::new(bin);
        command.args(["--user", "--scope", "--quiet", "--collect"]);
        if let Some(max) = memory_max {
            command.arg("-p").arg(format!("MemoryMax={max}"));
        }
        command.arg("--").arg("sh").arg("-c").arg(cmd);
        Ok(command)
    }
    #[cfg(not(unix))]
    {
        if isolate || memory_max.is_some() {
            return Err("isolate/memory_max is only supported on unix (systemd-run)".to_string());
        }
        Ok(shell_command(cmd))
    }
}

/// Build the system-shell command (`sh -c` unix, `cmd /C` windows).
fn shell_command(cmd: &str) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("sh");
        command.arg("-c").arg(cmd);
        command
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", cmd]);
        command
    }
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

/// Run `cmd` under the system shell, polling to `timeout` — on
/// timeout the child is killed and `rc` reads `-1`
/// (`result.status ?? -1`). Pipes drain on reader threads so a chatty
/// child can't deadlock the poll. Returns `(rc, stdout, stderr)`;
/// `Err` mirrors `result.error`.
///
/// The child sees a sanitized environment
/// ([`peregrin_core::child_env`]) plus `params.env` — never the
/// daemon's whole env, which routinely carries secrets a campaign
/// must not casually inherit.
fn spawn_shell(
    cmd: &str,
    env: &Map<String, Value>,
    timeout: Duration,
    cwd: Option<&str>,
    isolate: bool,
    memory_max: Option<&str>,
) -> Result<(i64, String, String), String> {
    let mut command = spawn_command(cmd, isolate, memory_max)?;
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    command
        .env_clear()
        .envs(peregrin_core::child_env())
        .envs(
            env.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = command.spawn().map_err(|e| e.to_string())?;
    let stdout_handle = child.stdout.take().map(read_pipe);
    let stderr_handle = child.stderr.take().map(read_pipe);
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    break child.wait().ok();
                }
                thread::sleep(POLL);
            }
            Err(e) => {
                let _ = child.kill();
                return Err(e.to_string());
            }
        }
    };
    let stdout = stdout_handle.map_or_else(String::new, |h| h.join().unwrap_or_default());
    let stderr = stderr_handle.map_or_else(String::new, |h| h.join().unwrap_or_default());
    // `result.status ?? -1` — a killed child has no exit code.
    let rc = status.and_then(|s| s.code()).map_or(-1, i64::from);
    Ok((rc, stdout, stderr))
}

/// The `emit` map lookup — `emit[rc]` then `emit.default` (TS
/// `Object.hasOwn` semantics).
fn emit_mapped(emit: &Map<String, Value>, rc: i64) -> Option<String> {
    emit.get(&rc.to_string())
        .or_else(|| emit.get("default"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

impl Driver for ShellDriver {
    fn name(&self) -> &'static str {
        "shell"
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
        let cmd = params
            .get("cmd")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if cmd.is_empty() {
            return Err(DriverError::new(format!(
                "shell node '{}' missing 'params.cmd'",
                node.id
            )));
        }

        if ctx.dry_run {
            return Ok(vec![Event::new(
                "cmd.done",
                serde_json::json!({ "cmd": cmd, "dryRun": true, "stdout": "" }),
                &node.id,
            )]);
        }

        let env = params
            .get("env")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let timeout = params
            .get("timeout")
            .and_then(Value::as_f64)
            .filter(|s| s.is_finite() && *s > 0.0)
            .map_or(TIMEOUT, |s| Duration::from_secs_f64(s).min(MAX_TIMEOUT));
        let cwd = params
            .get("cwd")
            .and_then(Value::as_str)
            .filter(|dir| !dir.is_empty());
        // Job isolation (PL-007 C4): `isolate: true` wraps the shell
        // in `systemd-run --user --scope`; `memory_max: "512M"` adds
        // `-p MemoryMax=` and implies isolation. Both fail loudly
        // when systemd user scopes aren't available.
        let isolate = params.get("isolate").is_some_and(js_truthy);
        let memory_max = params
            .get("memory_max")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let (rc, stdout, stderr) = match spawn_shell(cmd, &env, timeout, cwd, isolate, memory_max) {
            Err(error) => {
                return Ok(vec![Event::new(
                    "node.failed",
                    serde_json::json!({ "error": error, "cmd": cmd, "rc": -1 }),
                    &node.id,
                )]);
            }
            Ok(result) => result,
        };

        if let Some(mapped) = params
            .get("emit")
            .and_then(Value::as_object)
            .and_then(|emit| emit_mapped(emit, rc))
        {
            return Ok(vec![Event::new(
                mapped,
                serde_json::json!({ "stdout": stdout, "rc": rc, "cmd": cmd }),
                &node.id,
            )]);
        }

        if rc == 0 {
            return Ok(vec![Event::new(
                "cmd.done",
                serde_json::json!({ "stdout": stdout, "rc": rc, "cmd": cmd }),
                &node.id,
            )]);
        }
        Ok(vec![Event::new(
            "node.failed",
            serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "rc": rc,
                "cmd": cmd,
            }),
            &node.id,
        )])
    }
}
