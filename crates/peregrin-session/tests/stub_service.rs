#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `SepiaService` against a stub `sepia serve` — this same test binary
//! re-executed with `PEREGRIN_STUB_SERVICE` set (`spawn_one` always
//! passes `serve` as argv, which the libtest harness treats as a name
//! filter — selecting `serve` below, the only test here matching).
//! The stub speaks ndjson JSON-RPC on stdio: enough protocol to drive
//! the error-map, respawn, and notification-filter paths a real
//! service makes awkward to reach (bad manifests, error `data`
//! shapes, `session.event` fan-out).
//!
//! `sepia_binary`'s env-resolution arms run in re-exec'd subprocesses
//! too — edition-2024 `env::set_var` is `unsafe` (forbidden), so each
//! arm gets a child process with the env carved to hit exactly it.

use std::io::{BufRead, BufReader, Write};
use std::process::Command;

use peregrin_session::{SepiaService, ServicePlane, SessionPlane, Spec, sepia_binary};
use serde_json::{Value, json};

/// The stub entry point — runs the ndjson loop when re-executed as the
/// "service" (`PEREGRIN_STUB_SERVICE=1`); a no-op pass in the normal
/// suite. Writes go straight to fd 1 — `println!` would be captured
/// by libtest; `io::stdout()` bypasses that capture.
#[test]
fn serve() {
    if std::env::var_os("PEREGRIN_STUB_SERVICE").is_none() {
        return;
    }
    let proto: u64 = std::env::var("STUB_PROTO_FILE")
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);
    let bad_manifest = std::env::var_os("STUB_MANIFEST_BAD").is_some();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut emit = |v: Value| {
        writeln!(out, "{v}").unwrap();
        out.flush().unwrap();
    };
    // `eprintln!` is libtest-captured; a direct fd-2 write reaches the
    // parent's stderr ring (`stderr_tail`).
    std::io::stderr()
        .write_all(b"stub sepia serve up\n")
        .unwrap();
    for line in BufReader::new(stdin.lock()).lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(method) = req.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = req.get("id").and_then(Value::as_u64);
        let params = req.get("params").cloned().unwrap_or(Value::Null);
        let sid = params.get("id").and_then(Value::as_str).unwrap_or("?");
        match method {
            "manifest" => emit(json!({
                "jsonrpc": "2.0", "id": id,
                "result": if bad_manifest {
                    json!("not a manifest")
                } else {
                    json!({"name":"sepia","version":"0.0.0","protocol":proto,"methods":["m"]})
                }
            })),
            "sessions.subscribe" => {
                emit(json!({"jsonrpc":"2.0","id":id,"result":{"subscribed":true}}));
                // Wrong session → filtered out; unrelated method →
                // filtered out; bad events → parse-fail arm; one real
                // event → delivered to the subscriber.
                emit(json!({"jsonrpc":"2.0","method":"session.event","params":{
                    "sessionId":"other",
                    "events":[{"type":"runStarted","thread_id":"t","run_id":"r"}]}}));
                emit(json!({"jsonrpc":"2.0","method":"unrelated","params":{}}));
                emit(json!({"jsonrpc":"2.0","method":"session.event","params":{
                    "sessionId":sid,"events":[{"bogus":1}]}}));
                emit(json!({"jsonrpc":"2.0","method":"session.event","params":{
                    "sessionId":sid,
                    "events":[{"type":"runStarted","thread_id":"t","run_id":"r"}]}}));
            }
            "shutdown" => {
                emit(json!({"jsonrpc":"2.0","id":id,"result":{}}));
                out.flush().unwrap();
                std::process::exit(0);
            }
            "fail.plain" => emit(json!({
                "jsonrpc":"2.0","id":id,
                "error":{"code":-32000,"message":"boom"}})),
            "fail.withcode" => emit(json!({
                "jsonrpc":"2.0","id":id,
                "error":{"code":-32000,"message":"boom","data":{"code":"locked"}}})),
            "fail.baddata" => emit(json!({
                "jsonrpc":"2.0","id":id,
                "error":{"code":-32000,"message":"boom","data":{"code":42,"cause":"kaboom"}}})),
            "sessions.get" if sid == "badshape" => emit(json!({
                "jsonrpc":"2.0","id":id,"result":{"definitely":"not a session"}})),
            _ => emit(json!({"jsonrpc":"2.0","id":id,"result":{}})),
        }
    }
}

fn stub(extra: &[(&str, String)]) -> Spec {
    let mut env = vec![("PEREGRIN_STUB_SERVICE".into(), "1".into())];
    env.extend(extra.iter().map(|(k, v)| ((*k).to_string(), v.clone())));
    Spec {
        binary: std::env::current_exe().unwrap(),
        env,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stub_drives_full_surface() {
    let service = SepiaService::spawn(stub(&[])).await.unwrap();
    let plane = ServicePlane(service);

    // Typed calls decode fine over the stub.
    let _ = plane.get_session("s1", None).await;
    // `badshape` → the decode-error arm (`serde` rejects the shape).
    let err = plane.get_session("badshape", None).await.unwrap_err();
    assert_eq!(err.code, sepia_session::ControlErrorCode::Internal);
    // Raw calls hit `to_control`'s `data` arms: absent, parseable
    // `code`, and unparseable `code` with a `cause`.
    let e = plane.0.call("fail.plain", json!({})).await.unwrap_err();
    assert_eq!(e.data, None);
    let e = plane.0.call("fail.withcode", json!({})).await.unwrap_err();
    assert!(e.data.is_some());
    let e = plane.0.call("fail.baddata", json!({})).await.unwrap_err();
    assert!(e.data.is_some());
    // `subscribe` fans `session.event` notifications out to the
    // per-session channel — the stub emits one matching batch.
    let mut rx = plane.subscribe("s1", None).await.unwrap();
    let events = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("event batch timed out")
        .expect("channel closed");
    assert_eq!(events.len(), 1);
    // Respawn: `shutdown` exits the child → the next call respawns.
    plane.0.shutdown().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    plane.0.sessions(false).await.unwrap();
    // stderr tail picked the banner line up.
    assert!(
        plane
            .0
            .stderr_tail()
            .await
            .iter()
            .any(|l| l.contains("stub"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawn_protocol_mismatch_fails() {
    let dir = tempfile::tempdir().unwrap();
    let proto = dir.path().join("proto");
    std::fs::write(&proto, "999").unwrap();
    let spec = stub(&[("STUB_PROTO_FILE", proto.display().to_string())]);
    let err = SepiaService::spawn(spec).await.err().unwrap();
    assert!(err.message.contains("protocol"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn respawn_protocol_mismatch_fails() {
    let dir = tempfile::tempdir().unwrap();
    let proto = dir.path().join("proto");
    std::fs::write(&proto, "1").unwrap();
    let spec = stub(&[("STUB_PROTO_FILE", proto.display().to_string())]);
    let service = SepiaService::spawn(spec).await.unwrap();
    service.shutdown().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    // The respawned service reports a different protocol → surfaced.
    std::fs::write(&proto, "2").unwrap();
    let err = service.sessions(false).await.unwrap_err();
    assert!(err.message.contains("respawned"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bad_manifest_and_missing_binary_fail() {
    // Garbage manifest payload → the decode arm of `handshake`.
    let spec = stub(&[("STUB_MANIFEST_BAD", "1".into())]);
    let err = SepiaService::spawn(spec).await.err().unwrap();
    assert!(err.message.contains("manifest"));
    // Unspawnable binary → `spawn_one`'s map_err.
    let spec = Spec {
        binary: "/definitely/not/a/binary".into(),
        env: Vec::new(),
    };
    let err = SepiaService::spawn(spec).await.err().unwrap();
    assert!(err.message.contains("spawn"));
    // Spawnable but exits instantly → the `call` inside handshake errs.
    let spec = Spec {
        binary: "/bin/false".into(),
        env: Vec::new(),
    };
    let _err = SepiaService::spawn(spec).await.err().unwrap();
}

/// Re-exec helper for `sepia_binary`: run `env_probe` in a child with
/// a carved environment; the child asserts against `STUB_EXPECT`
/// (a path, or the literal `NONE`).
fn probe_env(envs: &[(&str, &str)], remove: &[&str], expect: &str) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["env_probe", "--exact", "--nocapture"]);
    for key in remove {
        cmd.env_remove(key);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.env("STUB_EXPECT", expect);
    let status = cmd.status().unwrap();
    assert!(status.success(), "env_probe child failed for {envs:?}");
}

#[test]
fn env_probe() {
    let Some(expect) = std::env::var("STUB_EXPECT").ok() else {
        return; // normal suite run — the parent test drives this
    };
    let resolved = sepia_binary();
    if expect == "NONE" {
        assert!(
            resolved.is_none(),
            "expected no resolution, got {resolved:?}"
        );
    } else {
        assert_eq!(
            resolved.as_deref(),
            Some(std::path::Path::new(expect.as_str()))
        );
    }
}

#[test]
fn sepia_binary_resolution_arms() {
    let dir = tempfile::tempdir().unwrap();
    let t = dir.path();
    // Arm 1 — PEREGRIN_SEPIA wins over everything.
    probe_env(
        &[("PEREGRIN_SEPIA", "/x/explicit-sepia")],
        &["CARGO_MANIFEST_DIR", "TESTKIT_BIN_DIRS"],
        "/x/explicit-sepia",
    );
    // Arm 2 — CARGO_MANIFEST_DIR/../../../sepia/target/<profile>/sepia.
    let manifest = t.join("a/b/c");
    let nested = t.join("sepia/target/debug");
    std::fs::create_dir_all(&manifest).unwrap();
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("sepia"), "x").unwrap();
    // `sepia_binary` returns the unnormalized `..` join — compare
    // against exactly that.
    let expect = manifest.join("../../../sepia/target/debug/sepia");
    probe_env(
        &[("CARGO_MANIFEST_DIR", manifest.to_str().unwrap())],
        &["PEREGRIN_SEPIA", "TESTKIT_BIN_DIRS"],
        expect.to_str().unwrap(),
    );
    // Arm 3 — TESTKIT_BIN_DIRS.
    let bins = t.join("bins");
    std::fs::create_dir_all(&bins).unwrap();
    std::fs::write(bins.join("sepia"), "x").unwrap();
    probe_env(
        &[("TESTKIT_BIN_DIRS", bins.to_str().unwrap())],
        &["PEREGRIN_SEPIA", "CARGO_MANIFEST_DIR"],
        bins.join("sepia").to_str().unwrap(),
    );
    // Arm 4 — $HOME/.cargo/shared-target/<profile>/sepia.
    let home = t.join("home");
    let shared = home.join(".cargo/shared-target/debug");
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(shared.join("sepia"), "x").unwrap();
    probe_env(
        &[("HOME", home.to_str().unwrap())],
        &["PEREGRIN_SEPIA", "CARGO_MANIFEST_DIR", "TESTKIT_BIN_DIRS"],
        shared.join("sepia").to_str().unwrap(),
    );
    // Arm 5 — PATH fallback.
    let pbin = t.join("pbin");
    std::fs::create_dir_all(&pbin).unwrap();
    std::fs::write(pbin.join("sepia"), "x").unwrap();
    probe_env(
        &[("PATH", pbin.to_str().unwrap())],
        &[
            "PEREGRIN_SEPIA",
            "CARGO_MANIFEST_DIR",
            "TESTKIT_BIN_DIRS",
            "HOME",
        ],
        pbin.join("sepia").to_str().unwrap(),
    );
    // Nothing resolves → None.
    let empty = t.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    probe_env(
        &[("PATH", empty.to_str().unwrap())],
        &[
            "PEREGRIN_SEPIA",
            "CARGO_MANIFEST_DIR",
            "TESTKIT_BIN_DIRS",
            "HOME",
        ],
        "NONE",
    );
}
