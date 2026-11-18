#![allow(clippy::unwrap_used)]
//! `driver: ops` coverage — a stub `serve_argv` (`sh -c`) plays the
//! tamias-serve wire so tests never need a real `tamias` install.

use peregrin_driver::{Driver, ExecutionContext};
use peregrin_driver_ops::OpsDriver;
use serde_json::json;

fn node(params: serde_json::Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "ops".into(),
        params,
        on: vec![],
        ..Default::default()
    }
}

/// A `serve_argv` triple that answers the FIRST stdin line with a
/// canned response then exits — the one-shot wire contract.
fn stub_argv(response: &str) -> serde_json::Value {
    // `read` consumes the request line; `printf` emits the canned
    // envelope. Single-quoted heredoc-style script avoids quoting
    // the JSON.
    let script = format!("IFS= read -r _line; printf '%s\\n' '{response}'");
    json!(["sh", "-c", script])
}

fn ops_params(driver: &str, query: &str, response: &str) -> serde_json::Value {
    json!({
        "driver": driver,
        "query": query,
        "serve_argv": stub_argv(response),
    })
}

#[test]
fn missing_required_params_error() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    let err = d.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("params.driver"), "{err}");
    let err = d
        .invoke(&node(json!({"driver": "lasius"})), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("params.query"), "{err}");
}

#[test]
fn ok_envelope_emits_result() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    // The serve returns the driver's {ok,result} envelope as the op
    // result — the driver unwraps one layer to the QueryResult.
    let out = d
        .invoke(
            &node(ops_params(
                "lasius",
                "cli.forge.pr.merge",
                r#"{"ok":true,"result":{"ok":true,"result":{"query":"cli.forge.pr.merge","data":{"merged":true}}}}"#,
            )),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "ops.done");
    assert_eq!(out[0].payload["driver"], "lasius");
    assert_eq!(out[0].payload["query"], "cli.forge.pr.merge");
    assert_eq!(out[0].payload["result"]["data"]["merged"], true);
}

#[test]
fn ok_false_envelope_emits_failure() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    let out = d
        .invoke(
            &node(ops_params(
                "lasius",
                "cli.forge.pr.merge",
                r#"{"ok":false,"error":"driver 'lasius' query 'cli.forge.pr.merge' failed: boom"}"#,
            )),
            &[],
            &ctx,
        )
        .unwrap();
    // No `emits` declared → failure falls back to node.failed.
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"].as_str().unwrap().contains("boom"),
        "{:?}",
        out[0].payload
    );
}

#[test]
fn spawn_failure_emits_failure_not_error() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    let mut n = node(json!({
        "driver": "lasius",
        "query": "cli.forge.pr.merge",
        "serve_argv": ["definitely-not-a-real-binary-peregrin-test"],
    }));
    n.emits = vec!["merge.done".to_string(), "merge.failed".to_string()];
    let out = d.invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "merge.failed");
    assert!(out[0].payload["error"].as_str().unwrap().contains("spawn"));
}

#[test]
fn emits_and_emit_map_override_kinds() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    // emits[0] is the success kind; emits[1] the failure kind.
    let mut n = node(ops_params(
        "lasius",
        "q",
        r#"{"ok":true,"result":{"data":1}}"#,
    ));
    n.emits = vec!["custom.done".to_string(), "custom.failed".to_string()];
    let out = d.invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "custom.done");
    // emit map beats emits[].
    let mut n = node(json!({
        "driver": "lasius",
        "query": "q",
        "serve_argv": stub_argv(r#"{"ok":false,"error":"nope"}"#),
        "emit": {"failure": "mapped.fail"},
    }));
    n.emits = vec!["a.done".to_string(), "a.failed".to_string()];
    let out = d.invoke(&n, &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "mapped.fail");
}

#[test]
fn echo_copies_payload_fields() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    let trigger = peregrin_core::Event::new(
        "pr.merged",
        json!({"number": 7, "pull_request": {"head": {"sha": "abc123"}}}),
        "test",
    );
    let params = json!({
        "driver": "lasius",
        "query": "cli.forge.pr.merge",
        "serve_argv": stub_argv(r#"{"ok":true,"result":{"data":{"ok":true}}}"#),
        "echo": ["number", "pull_request.head.sha", "missing.path"],
    });
    let out = d.invoke(&node(params), &[trigger], &ctx).unwrap();
    assert_eq!(out[0].payload["number"], 7);
    assert_eq!(out[0].payload["pull_request"]["head"]["sha"], "abc123");
    assert!(out[0].payload.get("missing").is_none());
    // Echo applies on the failure path too — downstream nodes still
    // see the correlation fields.
    let params = json!({
        "driver": "lasius",
        "query": "cli.forge.pr.merge",
        "serve_argv": stub_argv(r#"{"ok":false,"error":"nope"}"#),
        "echo": ["number"],
    });
    let trigger = peregrin_core::Event::new("pr.merged", json!({"number": 9}), "test");
    let out = d.invoke(&node(params), &[trigger], &ctx).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["number"], 9);
}

#[test]
fn dry_run_does_not_spawn() {
    let d = OpsDriver::new();
    let ctx = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = d
        .invoke(
            &node(json!({
                "driver": "lasius",
                "query": "q",
                "serve_argv": ["definitely-not-a-real-binary-peregrin-test"],
            })),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "ops.done");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["driver"], "lasius");
}

#[test]
fn request_line_shape() {
    // Capture what the driver writes: the stub tees stdin to a file.
    let dir = tempfile::tempdir().unwrap();
    let capture = dir.path().join("req.jsonl");
    let script = format!(
        "IFS= read -r line; printf '%s' \"$line\" > {}; printf '%s\\n' '{{\"ok\":true,\"result\":{{\"data\":1}}}}'",
        capture.display()
    );
    let d = OpsDriver::new();
    let ctx = ExecutionContext::default();
    let params = json!({
        "driver": "lasius",
        "query": "cli.forge.pr.merge",
        "params": {"number": 7},
        "serve_argv": ["sh", "-c", script],
        "timeout": 5,
    });
    d.invoke(&node(params), &[], &ctx).unwrap();
    let sent: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&capture).unwrap()).unwrap();
    assert_eq!(sent["op"], "driver.query");
    assert_eq!(sent["driver"], "lasius");
    assert_eq!(sent["query"], "cli.forge.pr.merge");
    assert_eq!(sent["params"]["number"], 7);
    assert_eq!(sent["timeout_ms"], 5000);
}
