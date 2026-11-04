#![allow(clippy::unwrap_used)]
use peregrin_driver::{Driver, ExecutionContext};
use peregrin_driver_shell::ShellDriver;
use serde_json::json;

fn node(params: serde_json::Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "shell".into(),
        params,
        on: vec![],
        ..Default::default()
    }
}

#[test]
fn run_command_paths() {
    let d = ShellDriver::new();
    let ctx = ExecutionContext::default();
    // Missing params.cmd → DriverError (TS Effect.fail).
    let err = d.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("missing 'params.cmd'"));

    // rc 0 → cmd.done.
    let out = d.invoke(&node(json!({"cmd": "true"})), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "cmd.done");
    assert_eq!(out[0].payload["rc"], 0);

    // rc != 0 → node.failed with stderr.
    let out = d
        .invoke(&node(json!({"cmd": "exit 3"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["rc"], 3);

    // stdout lands in the payload.
    let out = d
        .invoke(&node(json!({"cmd": "printf hello"})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].payload["stdout"], "hello");

    // `emit` map: rc → event type, `default` fallback.
    let out = d
        .invoke(
            &node(json!({"cmd": "exit 1", "emit": {"1": "one.seen", "default": "d.seen"}})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "one.seen");
    let out = d
        .invoke(
            &node(json!({"cmd": "exit 7", "emit": {"1": "one.seen", "default": "d.seen"}})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "d.seen");

    // dry_run short-circuits.
    let dry = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = d.invoke(&node(json!({"cmd": "true"})), &[], &dry).unwrap();
    assert_eq!(out[0].payload["dryRun"], true);
}

/// Re-exec guard: tests that need a marker var in the process env
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
fn spawned_child_does_not_inherit_ambient_env() {
    if reexec_with_marker() {
        return;
    }
    let d = ShellDriver::new();
    let ctx = ExecutionContext::default();
    // The marker rides this process's env but must not reach the
    // spawned `sh` — allowlisted basics still do.
    let out = d
        .invoke(
            &node(json!({"cmd": "printf %s \"[$PERE_TEST_SCRUB][$HOME]\" || true"})),
            &[],
            &ctx,
        )
        .unwrap();
    let stdout = out[0].payload["stdout"].as_str().unwrap();
    assert!(stdout.starts_with("[]"), "ambient env leaked: {stdout}");
}

#[test]
fn params_env_reaches_the_child() {
    let d = ShellDriver::new();
    let ctx = ExecutionContext::default();
    let out = d
        .invoke(
            &node(json!({"cmd": "printf %s \"$MY_DECLARED\"", "env": {"MY_DECLARED": "vv"}})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].payload["stdout"], "vv");
    // `params.env` can also *set* a marker an allowlist name — declared
    // wins, it's explicit yaml.
    let out = d
        .invoke(
            &node(json!({"cmd": "printf %s \"[$PERE_TEST_SCRUB]\"", "env": {"PERE_TEST_SCRUB": "ok"}})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].payload["stdout"], "[ok]");
}

#[test]
fn params_cwd_sets_working_dir() {
    let d = ShellDriver::new();
    let ctx = ExecutionContext::default();
    let dir = tempfile::TempDir::new().unwrap();
    let out = d
        .invoke(&node(json!({"cmd": "pwd", "cwd": dir.path()})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "cmd.done");
    // `pwd` may print a symlink-resolved path — canonicalize both.
    let stdout = out[0].payload["stdout"].as_str().unwrap();
    assert_eq!(
        std::fs::canonicalize(stdout.trim()).unwrap(),
        std::fs::canonicalize(dir.path()).unwrap()
    );
    // An empty cwd is ignored; a bogus one fails the spawn.
    let out = d
        .invoke(&node(json!({"cmd": "pwd", "cwd": ""})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "cmd.done");
    // Schemas: `cwd` is declared optional in the input contract.
    let input = d.input().unwrap();
    assert_eq!(input.fields["cwd"], serde_json::json!("string?"));
    assert_eq!(d.output().len(), 2);
    let out = d
        .invoke(
            &node(json!({"cmd": "true", "cwd": "/definitely/not/a/dir"})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
}

#[test]
fn params_timeout_is_honored() {
    let d = ShellDriver::new();
    let ctx = ExecutionContext::default();
    let out = d
        .invoke(&node(json!({"cmd": "sleep 5", "timeout": 0.2})), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["rc"], -1);
}

#[test]
fn oversized_output_is_capped_not_unbounded() {
    let d = ShellDriver::new();
    let ctx = ExecutionContext::default();
    // ~12 MiB of output: capture must truncate at 10 MiB while the
    // child still completes (rc 0, cmd.done).
    let out = d
        .invoke(
            &node(json!({"cmd": "head -c 12000000 /dev/zero | tr '\\0' 'x'"})),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "cmd.done");
    let stdout = out[0].payload["stdout"].as_str().unwrap();
    assert_eq!(stdout.len(), 10 * 1024 * 1024);
}
