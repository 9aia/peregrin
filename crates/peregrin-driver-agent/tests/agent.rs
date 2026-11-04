#![allow(clippy::unwrap_used)]
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use peregrin_driver::{Driver, ExecutionContext};
use peregrin_driver_agent::AgentDriver;
use peregrin_template::{Effects, SystemEffects};
use serde_json::{Value, json};

fn node(params: Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "agent".into(),
        params,
        ..Default::default()
    }
}

/// cwd/read_file under test control — covers the `process.cwd()`-joined
/// `prompt_file` branch without touching the process cwd.
struct DirEffects {
    root: PathBuf,
}

impl Effects for DirEffects {
    fn env(&self, name: &str) -> Option<String> {
        SystemEffects.env(name)
    }
    fn cwd(&self) -> PathBuf {
        self.root.clone()
    }
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }
    fn shell(&self, command: &str) -> Result<String, String> {
        SystemEffects.shell(command)
    }
    fn now_unix(&self) -> i64 {
        0
    }
    fn now_iso(&self) -> String {
        String::new()
    }
    fn random(&self) -> f64 {
        0.5
    }
    fn uuid(&self) -> String {
        "u".to_string()
    }
}

/// One-shot HTTP stub for the LiteLLM path — same shape as the
/// http-driver stub.
struct Stub {
    url: String,
    request: mpsc::Receiver<String>,
    join: thread::JoinHandle<()>,
}

fn request_complete(buf: &[u8]) -> bool {
    let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&buf[..pos]);
    let len = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::trim)
                .and_then(|v| v.parse::<usize>().ok())
        })
        .unwrap_or(0);
    buf.len() >= pos + 4 + len
}

/// Respond `status` + optional content-type + body; content-length is
/// computed so the payload can't drift from the header.
fn serve(status: &'static str, content_type: Option<&'static str>, body: &'static str) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    let join = thread::spawn(move || {
        let content_type = content_type.map_or(String::new(), |t| format!("content-type: {t}\r\n"));
        let response = format!(
            "HTTP/1.1 {status}\r\n{content_type}content-length: {}\r\n\r\n{body}",
            body.len()
        );
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = Vec::new();
        let mut tmp = [0_u8; 8192];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    if request_complete(&buf) {
                        break;
                    }
                }
            }
        }
        let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    Stub {
        url,
        request: rx,
        join,
    }
}

impl Stub {
    fn captured(self) -> String {
        let request = self.request.recv_timeout(Duration::from_secs(5)).unwrap();
        let _ = self.join.join();
        request
    }
}

fn invoke(node: &peregrin_core::Node, ctx: &ExecutionContext) -> Vec<peregrin_core::Event> {
    AgentDriver::new().invoke(node, &[], ctx).unwrap()
}

#[test]
fn name_and_param_errors() {
    let driver = AgentDriver::new();
    let _ = AgentDriver::default();
    assert_eq!(driver.name(), "agent");
    let ctx = ExecutionContext::default();

    let err = driver.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("missing 'params.prompt'"));

    let err = driver
        .invoke(
            &node(json!({"prompt_file": "/nonexistent/x.md"})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("failed to read prompt_file"));

    // Resolution failure under strict is a DriverError.
    let strict = ExecutionContext {
        strict: true,
        ..ExecutionContext::default()
    };
    let err = driver
        .invoke(&node(json!({"prompt": "${nope.thing}"})), &[], &strict)
        .unwrap_err();
    assert!(err.reason.contains("unresolved placeholders"));

    // Runner validation.
    let err = driver
        .invoke(&node(json!({"prompt": "x", "runner": "bogus"})), &[], &ctx)
        .unwrap_err();
    assert_eq!(err.reason, "unknown agent runner 'bogus'");
    let err = driver
        .invoke(&node(json!({"prompt": "x", "runner": "custom"})), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("missing 'params.binary'"));
    let err = driver
        .invoke(
            &node(json!({"prompt": "x", "runner": "custom", "binary": "cat"})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(
        err.reason
            .contains("'params.flags' must be an array of strings")
    );
    let err = driver
        .invoke(
            &node(json!({"prompt": "x", "runner": "custom", "binary": "cat", "flags": ["a", 1]})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(
        err.reason
            .contains("'params.flags' must be an array of strings")
    );
    let err = driver
        .invoke(
            &node(
                json!({"prompt": "x", "runner": "custom", "binary": "cat", "flags": [], "extra_args": "nope"}),
            ),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(
        err.reason
            .contains("'params.extra_args' must be an array of strings")
    );
}

#[test]
fn custom_runner_pipes_prompt_to_stdin() {
    let ctx = ExecutionContext::default();
    // cat echoes the stdin prompt back as stdout.
    let out = invoke(
        &node(json!({
            "prompt": "hello agent",
            "runner": "custom",
            "binary": "cat",
            "flags": [],
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "n.done"); // no emits → `{id}.done`
    assert_eq!(out[0].payload["stdout"], "hello agent");
    assert_eq!(out[0].payload["rc"], 0);
    assert_eq!(out[0].payload["runner"], "custom");
}

#[test]
fn custom_runner_event_routing() {
    let ctx = ExecutionContext::default();

    // emits list: rc 0 → emits[0], nonzero → emits[1].
    let mut n = node(json!({
        "prompt": "x", "runner": "custom", "binary": "true", "flags": [],
    }));
    n.emits = vec!["a.ok".into(), "a.bad".into()];
    let out = invoke(&n, &ctx);
    assert_eq!(out[0].kind, "a.ok");

    let mut n = node(json!({
        "prompt": "x", "runner": "custom", "binary": "false", "flags": [],
    }));
    n.emits = vec!["a.ok".into(), "a.bad".into()];
    let out = invoke(&n, &ctx);
    assert_eq!(out[0].kind, "a.bad");

    // emits[1] absent → falls to emits[0].
    let mut n = node(json!({
        "prompt": "x", "runner": "custom", "binary": "false", "flags": [],
    }));
    n.emits = vec!["only".into()];
    let out = invoke(&n, &ctx);
    assert_eq!(out[0].kind, "only");

    // `emit` map: exact rc then `default`.
    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom", "binary": "sh",
            "flags": ["-c", "exit 2"], "emit": {"2": "two.seen", "default": "d.seen"},
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "two.seen");
    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom", "binary": "sh",
            "flags": ["-c", "exit 3"], "emit": {"2": "two.seen", "default": "d.seen"},
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "d.seen");

    // stderr lands in the payload on a mapped/failed run.
    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom", "binary": "sh",
            "flags": ["-c", "echo oops >&2; exit 1"],
        })),
        &ctx,
    );
    assert_eq!(out[0].payload["stderr"], "oops\n");
    assert_eq!(out[0].payload["rc"], 1);
}

#[test]
fn env_and_secrets_merge() {
    let ctx = ExecutionContext {
        secrets: std::rc::Rc::new(BTreeMap::from_iter([(
            "MY_SECRET".to_string(),
            "s3cret".to_string(),
        )])),
        ..ExecutionContext::default()
    };
    let out = invoke(
        &node(json!({
            "prompt": "x",
            "runner": "custom",
            "binary": "sh",
            "flags": ["-c", "printf %s \"$MY_VAR|$MY_SECRET|$MISSING\""],
            "env": {"MY_VAR": "vv"},
            "secrets": ["MY_SECRET", "ABSENT_SECRET"],
        })),
        &ctx,
    );
    assert_eq!(out[0].payload["stdout"], "vv|s3cret|");
    assert_eq!(out[0].kind, "n.done");
}

#[test]
fn spawn_error_and_timeout_emit_node_failed() {
    let ctx = ExecutionContext::default();

    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom",
            "binary": "definitely-not-a-real-binary-xyz", "flags": [],
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["rc"], -1);
    assert_eq!(out[0].payload["runner"], "custom");
    assert!(out[0].payload["error"].is_string());

    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom", "binary": "sleep",
            "flags": ["10"], "timeout": 0.2,
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["rc"], -1);
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("ETIMEDOUT")
    );
}

#[test]
fn extra_args_extend_and_empty_prompt_stdin_null() {
    let ctx = ExecutionContext::default();

    // extra_args append to flags — `sh -c "..."` here.
    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom", "binary": "sh",
            "flags": ["-c"], "extra_args": ["printf hi"],
        })),
        &ctx,
    );
    assert_eq!(out[0].payload["stdout"], "hi");

    // An empty prompt file leaves prompt_text "" → the spawn gets
    // `Stdio::null()` instead of a piped prompt.
    let file = tempfile::NamedTempFile::new().unwrap();
    let out = invoke(
        &node(json!({
            "prompt_file": file.path(), "runner": "custom",
            "binary": "true", "flags": [],
        })),
        &ctx,
    );
    assert_eq!(out[0].payload["rc"], 0);
}

#[test]
fn cwd_param_and_no_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ExecutionContext::default();
    // timeout: 0 → no limit; cwd controls the child's working dir.
    let out = invoke(
        &node(json!({
            "prompt": "x", "runner": "custom", "binary": "pwd",
            "flags": [], "cwd": dir.path(), "timeout": 0,
        })),
        &ctx,
    );
    assert_eq!(
        out[0].payload["stdout"].as_str().unwrap().trim_end(),
        dir.path().to_str().unwrap()
    );
}

#[test]
fn prompt_file_paths() {
    let ctx = ExecutionContext::default();

    // Absolute prompt_file — contents become the stdin prompt.
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "from file").unwrap();
    let out = invoke(
        &node(json!({
            "prompt_file": file.path(), "runner": "custom",
            "binary": "cat", "flags": [],
        })),
        &ctx,
    );
    assert_eq!(out[0].payload["stdout"], "from file");

    // Relative prompt_file joins against effects.cwd().
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rel.md"), "relative prompt").unwrap();
    let driver = AgentDriver::with_effects(std::sync::Arc::new(DirEffects {
        root: dir.path().to_path_buf(),
    }));
    let out = driver
        .invoke(
            &node(json!({
                "prompt_file": "rel.md", "runner": "custom",
                "binary": "cat", "flags": [],
            })),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].payload["stdout"], "relative prompt");
}

#[test]
fn devin_runner_dry_run() {
    let dry = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let mut n = node(json!({
        "prompt": "do the thing",
        "permission_mode": "plan",
        "trust": true,
    }));
    n.emits = vec!["d.ok".into()];
    let out = invoke(&n, &dry);
    assert_eq!(out[0].kind, "d.ok");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["rc"], 0);
    assert_eq!(out[0].payload["runner"], "devin");
    let cmd = out[0].payload["cmd"].as_str().unwrap();
    assert_eq!(
        cmd,
        "-p do the thing --permission-mode plan --respect-workspace-trust true"
    );

    // prompt_file flips the argv to --prompt-file.
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "x").unwrap();
    let out = invoke(
        &node(json!({"prompt_file": file.path(), "trust": false})),
        &dry,
    );
    let cmd = out[0].payload["cmd"].as_str().unwrap();
    assert!(cmd.starts_with("-p --prompt-file "));
    assert!(cmd.ends_with("--respect-workspace-trust false"));
}

#[test]
fn litellm_completion() {
    let stub = serve(
        "200 OK",
        Some("application/json"),
        "{\"choices\":[{\"message\":{\"content\":\"done!\"}}]}",
    );
    let ctx = ExecutionContext::default();
    let out = invoke(
        &node(json!({
            "prompt": "the question",
            "model": "gpt-4o",
            "provider": "openai",
            "endpoint": stub.url,
            "api_key": "k1",
            "system": "be terse",
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "agent.litellm");
    assert_eq!(out[0].payload["content"], "done!");
    assert_eq!(out[0].payload["model"], "gpt-4o");
    assert_eq!(out[0].payload["provider"], "openai");
    let request = stub.captured();
    assert!(request.contains("authorization: Bearer k1"));
    assert!(request.contains("\"model\":\"openai/gpt-4o\""));
    assert!(request.contains("\"role\":\"system\""));
    assert!(request.contains("\"content\":\"the question\""));
}

#[test]
fn litellm_api_key_from_env_and_dry_run() {
    let stub = serve(
        "200 OK",
        Some("application/json"),
        "{\"choices\":[{\"message\":{\"content\":\"ok\"}}]}",
    );
    let ctx = ExecutionContext {
        secrets: std::rc::Rc::new(BTreeMap::from_iter([(
            "LITELLM_API_KEY".to_string(),
            "secret-key".to_string(),
        )])),
        ..ExecutionContext::default()
    };
    // `secrets` pulls LITELLM_API_KEY into env; it becomes the api key.
    let out = invoke(
        &node(json!({
            "prompt": "q", "model": "m", "endpoint": stub.url,
            "secrets": ["LITELLM_API_KEY"],
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "agent.litellm");
    let request = stub.captured();
    assert!(request.contains("authorization: Bearer secret-key"));
    assert!(request.contains("\"model\":\"m\""));

    // Dry run short-circuits before any request.
    let dry = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = invoke(
        &node(json!({"prompt": "q", "model": "m", "provider": "p"})),
        &dry,
    );
    assert_eq!(out[0].kind, "agent.litellm");
    assert_eq!(out[0].payload["content"], "");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["provider"], "p");
}

#[test]
fn litellm_failures() {
    let ctx = ExecutionContext::default();

    // Non-2xx → DriverError "failed with status".
    let stub = serve("500 ERR", None, "");
    let err = AgentDriver::new()
        .invoke(
            &node(json!({"prompt": "q", "model": "m", "endpoint": stub.url})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert_eq!(err.reason, "litellm request failed with status 500");
    stub.captured();

    // Transport failure.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    };
    let err = AgentDriver::new()
        .invoke(
            &node(json!({"prompt": "q", "model": "m", "endpoint": format!("http://{closed}/x")})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("litellm request failed for model 'm'"));

    // Unparseable response.
    let stub = serve("200 OK", None, "not json");
    let err = AgentDriver::new()
        .invoke(
            &node(json!({"prompt": "q", "model": "m", "endpoint": stub.url})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("failed to parse litellm response"));
    stub.captured();

    // Missing choices[0].message.content.
    let stub = serve("200 OK", None, "{\"choices\":[]}");
    let err = AgentDriver::new()
        .invoke(
            &node(json!({"prompt": "q", "model": "m", "endpoint": stub.url})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert_eq!(
        err.reason,
        "litellm response missing choices[0].message.content"
    );
    stub.captured();

    // An unusable api_key header value fails request construction.
    let err = AgentDriver::new()
        .invoke(
            &node(json!({"prompt": "q", "model": "m", "api_key": "bad\nkey"})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("litellm request failed for model 'm'"));
}

/// Re-exec guard: tests needing a marker var in the process env
/// respawn the test binary with it set (edition-2024 `set_var` is
/// unsafe — the child process env is the clean seam).
fn reexec_with_marker() -> bool {
    if std::env::var_os("PERE_TEST_SCRUB").is_some() {
        return false;
    }
    let exe = std::env::current_exe().unwrap();
    let test = std::thread::current().name().unwrap().to_string();
    let out = std::process::Command::new(exe)
        .args([test.as_str(), "--exact", "--nocapture"])
        .env("PERE_TEST_SCRUB", "zzz-secret")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inner run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    true
}

#[test]
fn spawned_runner_does_not_inherit_ambient_env() {
    if reexec_with_marker() {
        return;
    }
    let out = AgentDriver::new()
        .invoke(
            &node(json!({
                "prompt": "x", "runner": "custom",
                "binary": "sh", "flags": ["-c", "printf %s \"[$PERE_TEST_SCRUB]\""],
            })),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert_eq!(out[0].payload["stdout"], "[]");
}

#[test]
fn declared_env_and_secrets_still_reach_the_runner() {
    let ctx = ExecutionContext {
        secrets: std::rc::Rc::new(BTreeMap::from_iter([(
            "MY_SECRET".to_string(),
            "s3cret".to_string(),
        )])),
        ..ExecutionContext::default()
    };
    let out = AgentDriver::new()
        .invoke(
            &node(json!({
                "prompt": "x", "runner": "custom",
                "binary": "sh", "flags": ["-c", "printf %s \"$MY_VAR|$MY_SECRET|$PERE_TEST_SCRUB\""],
                "env": {"MY_VAR": "vv"},
                "secrets": ["MY_SECRET"],
            })),
            &[],
            &ctx,
        )
        .unwrap();
    // Declared channels still work; the ambient marker stays out.
    assert_eq!(out[0].payload["stdout"], "vv|s3cret|");
}

// --- MockAgentDriver -------------------------------------------------

use peregrin_driver_agent::MockAgentDriver;
use std::sync::Arc;

fn mock_node(params: Value) -> peregrin_core::Node {
    peregrin_core::Node {
        driver: "mock-agent".into(),
        ..node(params)
    }
}

#[test]
fn mock_emits_agent_done_with_canned_response_and_prompt() {
    let out = MockAgentDriver::new()
        .invoke(
            &mock_node(json!({"prompt": "ship it", "response": "done:42"})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "agent.done");
    assert_eq!(out[0].payload["stdout"], "done:42");
    assert_eq!(out[0].payload["prompt"], "ship it");
    assert_eq!(out[0].payload["rc"], 0);
    assert_eq!(out[0].payload["runner"], "mock");
}

#[test]
fn mock_fail_emits_node_failed_with_message() {
    let out = MockAgentDriver::new()
        .invoke(
            &mock_node(json!({"prompt": "x", "fail": true, "fail_message": "planned outage"})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["error"], "planned outage");
    assert_eq!(out[0].payload["rc"], 1);
}

#[test]
fn mock_fail_default_message() {
    let out = MockAgentDriver::new()
        .invoke(
            &mock_node(json!({"prompt": "x", "fail": true})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert_eq!(out[0].payload["error"], "mock-agent scripted failure");
}

#[test]
fn mock_prompt_file_resolves_through_effects() {
    let dir = std::env::temp_dir().join(format!("peregrin-mock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("prompt.txt"), "from file").unwrap();
    let driver = MockAgentDriver::with_effects(Arc::new(DirEffects { root: dir.clone() }));
    let out = driver
        .invoke(
            &mock_node(json!({"prompt_file": "prompt.txt"})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert_eq!(out[0].payload["prompt"], "from file");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mock_prompt_file_read_error_surfaces() {
    let dir = PathBuf::from("/nonexistent-dir-xyz");
    let driver = MockAgentDriver::with_effects(Arc::new(DirEffects { root: dir }));
    let err = MockAgentDriver::with_effects(Arc::new(DirEffects {
        root: PathBuf::from("/nonexistent-dir-xyz"),
    }))
    .invoke(
        &mock_node(json!({"prompt_file": "missing.txt"})),
        &[],
        &ExecutionContext::default(),
    )
    .unwrap_err();
    let _ = driver;
    assert!(err.to_string().contains("prompt_file"), "{err}");
}

#[test]
fn mock_no_prompt_is_an_empty_prompt_not_an_error() {
    let out = MockAgentDriver::new()
        .invoke(&mock_node(json!({})), &[], &ExecutionContext::default())
        .unwrap();
    assert_eq!(out[0].kind, "agent.done");
    assert_eq!(out[0].payload["prompt"], "");
}

#[test]
fn mock_delay_ms_sleeps() {
    let started = std::time::Instant::now();
    MockAgentDriver::new()
        .invoke(
            &mock_node(json!({"prompt": "x", "delay_ms": 80})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(70));
}

#[test]
fn mock_absolute_prompt_file_path_used_verbatim() {
    let dir = std::env::temp_dir().join(format!("peregrin-mock-abs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("abs.txt");
    std::fs::write(&file, "absolute").unwrap();
    let out = MockAgentDriver::new()
        .invoke(
            &mock_node(json!({"prompt_file": file.to_str().unwrap()})),
            &[],
            &ExecutionContext::default(),
        )
        .unwrap();
    assert_eq!(out[0].payload["prompt"], "absolute");
    std::fs::remove_dir_all(&dir).unwrap();
}
