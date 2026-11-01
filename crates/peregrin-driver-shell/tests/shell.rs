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
