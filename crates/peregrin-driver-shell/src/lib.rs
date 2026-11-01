//! peregrin-driver-shell — `driver: shell`, port of `ac-driver-shell`.
//! `params.cmd` runs under the system shell (`spawnSync(cmd,
//! {shell: true, timeout: 30000})` in the TS — 30 s hardcoded).
//! `params.emit` maps `rc` → event type; otherwise `rc == 0` emits
//! `cmd.done` and anything else `node.failed`.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{Context, Effects, Options, SystemEffects, resolve_value_opts};

/// `spawnSync`'s hardcoded timeout — 30 s (ADR-0005 default).
const TIMEOUT: Duration = Duration::from_secs(30);
/// Poll cadence while waiting on the child.
const POLL: Duration = Duration::from_millis(5);

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
        ctx: &ExecutionContext,
    ) -> Result<Map<String, Value>, DriverError> {
        let tctx = Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &*self.effects,
        };
        let resolved = resolve_value_opts(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &[],
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }
}

/// `input` contract — `cmd` required, `timeout`/`emit`/`env` optional
/// (the TS `input` EventSchema).
fn input_schema() -> EventSchema {
    EventSchema {
        kind: String::new(),
        fields: Map::from_iter([
            ("cmd".to_string(), Value::String("string".to_string())),
            ("timeout".to_string(), Value::String("integer?".to_string())),
            ("emit".to_string(), Value::String("object?".to_string())),
            ("env".to_string(), Value::String("object?".to_string())),
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

fn read_pipe(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut text = String::new();
        let _ = pipe.read_to_string(&mut text);
        text
    })
}

/// Run `cmd` under the system shell, polling to the 30 s deadline —
/// on timeout the child is killed and `rc` reads `-1`
/// (`result.status ?? -1`). Pipes drain on reader threads so a chatty
/// child can't deadlock the poll. Returns `(rc, stdout, stderr)`;
/// `Err` mirrors `result.error`.
fn spawn_shell(cmd: &str, env: &Map<String, Value>) -> Result<(i64, String, String), String> {
    let mut command = shell_command(cmd);
    command
        .envs(std::env::vars())
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
    let deadline = Instant::now() + TIMEOUT;
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
        _triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, ctx)?;
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
        let (rc, stdout, stderr) = match spawn_shell(cmd, &env) {
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
