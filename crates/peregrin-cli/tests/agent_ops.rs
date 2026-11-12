//! Agent-ops coverage (P-028/T-021) — the retired `peregrin-mcp`
//! tools as engine verbs, driven end-to-end through the real
//! `tamias-driver-peregrin` binary: `run <argv>` for the verbs and
//! the `serve` ndjson wire for `cli.<verb>` queries (the same path
//! `engine_ops::run_verb` takes).

#![allow(clippy::unwrap_used, clippy::pedantic)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

fn driver() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tamias-driver-peregrin"));
    cmd.env("PEREGRIN_HOME", roots().join("home"));
    cmd.env("PEREGRIN_CACHE", roots().join("cache"));
    // Bundle store/cache roots — tamias's chain (TAMIAS_HOME/store,
    // TAMIAS_CACHE) so `bundle.*` ops stay inside the sandbox.
    cmd.env("TAMIAS_HOME", roots().join("tamias-home"));
    cmd.env("TAMIAS_CACHE", roots().join("tamias-cache"));
    cmd
}

/// Per-test roots under the shared sandbox — tests run in parallel
/// threads and the tamias store's `.tmp-*` staging sweep is
/// store-wide, so every test gets its own home/store/cache or
/// concurrent `bundle install`s would delete each other's staging
/// dirs mid-commit.
fn roots() -> PathBuf {
    let name = std::thread::current()
        .name()
        .unwrap_or("test")
        .replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    sandbox().join(name)
}

/// One tempdir per test process — `sandbox()` memoizes it so every
/// spawned verb shares store/cache/home roots.
fn sandbox() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "peregrin-agent-ops-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    })
    .clone()
}

/// Run a verb, return `(stdout, stderr, code)`.
fn verb(args: &[&str]) -> (String, String, i32) {
    let out = driver().arg("run").args(args).output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn stdout_json(args: &[&str]) -> Value {
    let (out, err, code) = verb(args);
    assert_eq!(code, 0, "{args:?} failed: {err}\n{out}");
    serde_json::from_str(out.trim()).unwrap_or_else(|e| panic!("{args:?}: {e}\n{out}"))
}

/// A shell-driver workflow — `config.msg` + `secret.tok` land in
/// `<dir>/out.txt` (the MCP fixture's shape).
fn fixture_workflow(dir: &Path) -> PathBuf {
    let yaml = format!(
        "name: agent-fixture\n\
         capabilities:\n  secrets: true\n\
         nodes:\n\
         \x20 - id: write\n\
         \x20   driver: shell\n\
         \x20   on: [start]\n\
         \x20   emits: [write.done, write.failed]\n\
         \x20   params:\n\
         \x20     cmd: \"printf ${{config.msg}}${{secret.tok}} > {dir}/out.txt\"\n\
         \x20     emit:\n\
         \x20       \"0\": write.done\n\
         \x20       default: write.failed\n",
        dir = dir.display()
    );
    let path = dir.join("fixture.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

fn bundle_dir(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("tamias.yaml"),
        format!("name: {name}\nversion: \"0.1.0\"\ndescription: test bundle\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("main.yaml"),
        "name: bundled\nnodes:\n  - id: n\n    driver: noop\n    on: [start]\n",
    )
    .unwrap();
    dir
}

fn write_log(path: &Path, lines: &[&str]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

#[test]
fn bundle_verbs_lifecycle() {
    // Empty store lists cleanly.
    let list = stdout_json(&["bundle", "list"]);
    assert!(list["bundles"].is_array());

    let dir = bundle_dir(&sandbox(), "demo-bundle");
    let installed = stdout_json(&["bundle", "install", dir.to_str().unwrap()]);
    assert_eq!(installed["label"], "demo-bundle@0.1.0");

    let list = stdout_json(&["bundle", "list"]);
    assert!(
        list["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["name"] == "demo-bundle")
    );

    let info = stdout_json(&["bundle", "info", "demo-bundle"]);
    assert_eq!(info["description"], "test bundle");
    assert!(info["size_bytes"].as_u64().unwrap() > 0);

    // Error arms — unknown name, bad ref.
    let (_, err, code) = verb(&["bundle", "info", "ghost"]);
    assert_ne!(code, 0);
    assert!(!err.is_empty());
    let (_, _, code) = verb(&["bundle", "install", "definitely-not-a-ref"]);
    assert_ne!(code, 0);

    let removed = stdout_json(&["bundle", "remove", "demo-bundle"]);
    assert_eq!(removed["name"], "demo-bundle");
}

#[test]
fn logs_verbs_scan_tail_explain() {
    let logs = roots().join("logs-fixture");
    write_log(
        &logs.join("a.jsonl"),
        &[
            "{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\",\"ts\":1000}",
            "{\"type\":\"n.done\",\"payload\":{},\"source\":\"n\",\"ts\":1500}",
            "{\"type\":\"workflow.done\",\"payload\":{},\"source\":\"__runner__\",\"ts\":2000}",
        ],
    );
    write_log(
        &logs.join("nested").join("b.jsonl"),
        &[
            "{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\",\"ts\":100}",
            "{\"type\":\"node.failed\",\"payload\":{\"node\":\"x\",\"error\":\"boom\"},\"source\":\"x\",\"ts\":200}",
            "{\"type\":\"workflow.failed\",\"payload\":{\"node\":\"x\",\"error\":\"boom\"},\"source\":\"__runner__\",\"ts\":300}",
        ],
    );

    // `logs runs` — recursive scan, verdicts, newest first.
    let report = stdout_json(&["logs", "runs", "--dir", logs.to_str().unwrap()]);
    assert_eq!(report["total"], 2);
    let runs = report["runs"].as_array().unwrap();
    assert!(runs.iter().any(|r| r["verdict"] == "done"));
    assert!(runs.iter().any(|r| r["verdict"] == "failed"));

    // `logs events` — bounded tail + type filter.
    let a = logs.join("a.jsonl");
    let report = stdout_json(&["logs", "events", a.to_str().unwrap(), "--limit", "2"]);
    assert_eq!(report["total"], 3);
    assert_eq!(report["events"].as_array().unwrap().len(), 2);
    assert_eq!(report["events"][0]["type"], "n.done");
    let report = stdout_json(&[
        "logs",
        "events",
        a.to_str().unwrap(),
        "--type",
        "workflow.done",
    ]);
    assert_eq!(report["total"], 1);

    // `logs explain` — verdict, failures, duration.
    let report = stdout_json(&[
        "logs",
        "explain",
        logs.join("nested").join("b.jsonl").to_str().unwrap(),
    ]);
    assert_eq!(report["verdict"], "failed");
    assert_eq!(report["failures"][0]["node"], "x");
    assert_eq!(report["kinds"]["node.failed"], 1);

    // Error arms.
    let (_, _, code) = verb(&["logs", "runs", "--dir", "/nonexistent-dir"]);
    assert_ne!(code, 0);
    let (_, _, code) = verb(&["logs", "events", "/nonexistent.jsonl"]);
    assert_ne!(code, 0);
}

#[test]
fn campaign_validate_bundle_selector() {
    let dir = bundle_dir(&sandbox(), "validate-me");
    let out = stdout_json(&["bundle", "install", dir.to_str().unwrap()]);
    assert_eq!(out["name"], "validate-me");
    // `campaign validate --bundle` resolves main.yaml inside the entry.
    let (stdout, _, code) = verb(&["campaign", "validate", "--bundle", "validate-me"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("bundled"));
    // Missing both selectors → clean error.
    let (_, _, code) = verb(&["campaign", "validate"]);
    assert_ne!(code, 0);
}

#[test]
fn campaign_submit_status_events_flow() {
    let dir = roots().join("submit-fixture");
    std::fs::create_dir_all(&dir).unwrap();
    let wf = fixture_workflow(&dir);

    // submit — returns run_id + log immediately (never blocks).
    let submitted = stdout_json(&[
        "campaign",
        "submit",
        wf.to_str().unwrap(),
        "--config",
        "{\"msg\":\"hi\"}",
        "--secrets",
        "{\"tok\":\"!\"}",
    ]);
    let run_id = submitted["run_id"].as_str().unwrap().to_string();
    let log = PathBuf::from(submitted["log"].as_str().unwrap());
    // Default log location: <PEREGRIN_HOME>/runs/<run-id>.jsonl.
    assert_eq!(log.parent().unwrap(), roots().join("home").join("runs"));
    assert!(log.ends_with(format!("{run_id}.jsonl")));

    // Poll status until the run reaches a verdict (bounded wait).
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        let status = stdout_json(&["campaign", "status", &run_id]);
        if status["verdict"] != "pending" && status["verdict"] != "running" {
            break status;
        }
        assert!(Instant::now() < deadline, "run never finished: {status}");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(status["verdict"], "done", "{status}");
    assert!(status["events"].as_u64().unwrap() >= 2);

    // The config+secrets merge reached the shell driver.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if dir.join("out.txt").exists() {
            break;
        }
        assert!(Instant::now() < deadline, "out.txt never written");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(std::fs::read_to_string(dir.join("out.txt")).unwrap(), "hi!");

    // events — bounded tail of the run's log.
    let events = stdout_json(&["campaign", "events", &run_id, "--limit", "3"]);
    assert!(events["total"].as_u64().unwrap() >= 2);
    let tail = events["events"].as_array().unwrap();
    assert!(tail.len() <= 3);
    assert_eq!(tail.last().unwrap()["type"], "workflow.done");
    let typed = stdout_json(&["campaign", "events", &run_id, "--type", "workflow.done"]);
    assert_eq!(typed["total"], 1);

    // The log also shows up in `logs runs` under <home>/runs.
    let runs = stdout_json(&["logs", "runs"]);
    assert!(
        runs["runs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["path"].as_str().unwrap() == log.display().to_string())
    );

    // Error arms — bad selectors fail before any run id exists.
    let (_, _, code) = verb(&["campaign", "submit"]);
    assert_ne!(code, 0);
    let (_, _, code) = verb(&["campaign", "submit", "/nonexistent.yaml"]);
    assert_ne!(code, 0);
    let (_, _, code) = verb(&[
        "campaign",
        "submit",
        wf.to_str().unwrap(),
        "--secrets",
        "[1]",
    ]);
    assert_ne!(code, 0);
}

#[test]
fn campaign_submit_from_bundle() {
    let dir = bundle_dir(&sandbox(), "run-me");
    let installed = stdout_json(&["bundle", "install", dir.to_str().unwrap()]);
    assert_eq!(installed["name"], "run-me");
    let submitted = stdout_json(&["campaign", "submit", "--bundle", "run-me"]);
    let run_id = submitted["run_id"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = stdout_json(&["campaign", "status", &run_id]);
        if status["verdict"] == "done" || status["verdict"] == "failed" {
            assert_eq!(status["verdict"], "done", "{status}");
            break;
        }
        assert!(Instant::now() < deadline, "run never finished: {status}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

// --- the `serve` wire: `cli.<verb>` queries (engine_ops::run_verb) -----------

struct Serve {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl Serve {
    fn spawn() -> Self {
        let mut child = driver()
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// One `{"op":"query",…}` round-trip → the response `data`.
    fn query(&mut self, query: &str, params: Value) -> Value {
        let req = json!({"op": "query", "query": query, "params": params});
        writeln!(self.stdin, "{req}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let resp: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(resp["ok"], true, "{query}: {resp}");
        resp["result"]["data"].clone()
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn cli_ops_over_serve_wire() {
    let mut serve = Serve::spawn();

    // The ops manifest advertises the new verbs as cli.* queries.
    let manifest = serve.query("ops", json!({}));
    let ops = manifest["ops"].as_array().unwrap();
    for want in [
        "cli.bundle.install",
        "cli.bundle.remove",
        "cli.bundle.list",
        "cli.bundle.info",
        "cli.logs.runs",
        "cli.logs.events",
        "cli.logs.explain",
        "cli.campaign.validate",
        "cli.campaign.submit",
        "cli.campaign.status",
        "cli.campaign.events",
    ] {
        assert!(
            ops.iter().any(|o| o["query"] == json!(want)),
            "{want} missing from ops manifest"
        );
    }
    // `campaign exec` is hidden plumbing — never an op. The surfaces
    // are unchanged.
    assert!(!ops.iter().any(|o| o["query"] == json!("cli.campaign.exec")));
    for surface in [
        "serve",
        "campaign.run",
        "campaign.serve",
        "sessions.attach",
        "sessions.stream",
        "events",
    ] {
        assert!(
            manifest["surfaces"]
                .as_array()
                .unwrap()
                .contains(&json!(surface))
        );
    }

    // `cli.bundle.list` over the wire — run_verb re-execs `run` in a
    // captured child; code 0 + JSON stdout.
    let data = serve.query("cli.bundle.list", json!({}));
    assert_eq!(data["code"], 0, "{data}");
    let out: Value = serde_json::from_str(data["stdout"].as_str().unwrap()).unwrap();
    assert!(out["bundles"].is_array(), "{out}");

    // `cli.logs.events` — bounded tail over the fixture log.
    let logs = roots().join("logs-fixture");
    let a = logs.join("a.jsonl");
    write_log(
        &a,
        &[
            "{\"type\":\"n.run\",\"payload\":{},\"source\":\"n\",\"ts\":1000}",
            "{\"type\":\"n.done\",\"payload\":{},\"source\":\"n\",\"ts\":1500}",
            "{\"type\":\"workflow.done\",\"payload\":{},\"source\":\"__runner__\",\"ts\":2000}",
        ],
    );
    let data = serve.query(
        "cli.logs.events",
        json!({"path": a.to_str().unwrap(), "limit": 2, "type": "n.done"}),
    );
    assert_eq!(data["code"], 0, "{data}");
    let out: Value = serde_json::from_str(data["stdout"].as_str().unwrap()).unwrap();
    assert_eq!(out["total"], 1);
    assert_eq!(out["events"][0]["type"], "n.done");

    // `cli.campaign.status` — pending for an unknown run id.
    let data = serve.query("cli.campaign.status", json!({"run": "no-such-run"}));
    assert_eq!(data["code"], 0, "{data}");
    let out: Value = serde_json::from_str(data["stdout"].as_str().unwrap()).unwrap();
    assert_eq!(out["verdict"], "pending");

    // `cli.campaign.submit` over the wire — the full op path:
    // run_verb child → submit → detached exec grandchild → poll.
    let dir = roots().join("wire-submit");
    std::fs::create_dir_all(&dir).unwrap();
    let wf = fixture_workflow(&dir);
    let data = serve.query("cli.campaign.submit", json!({"path": wf.to_str().unwrap()}));
    assert_eq!(data["code"], 0, "{data}");
    let out: Value = serde_json::from_str(data["stdout"].as_str().unwrap()).unwrap();
    let run_id = out["run_id"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let data = serve.query("cli.campaign.status", json!({"run": run_id}));
        let status: Value = serde_json::from_str(data["stdout"].as_str().unwrap()).unwrap();
        if status["verdict"] == "done" || status["verdict"] == "failed" {
            assert_eq!(status["verdict"], "done", "{status}");
            break;
        }
        assert!(Instant::now() < deadline, "run never finished: {status}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Generated-MCP parity (feature `mcp`): pavo-driver-kit's surface
/// turns the ops manifest into MCP tools — `tools/list` must
/// advertise every retired-`peregrin-mcp` replacement and
/// `tools/call` must round-trip a bounded one. Compiled only when
/// the package's `mcp` feature is on (`cargo test --features mcp`).
#[cfg(feature = "mcp")]
#[test]
fn mcp_tools_surface() {
    let mut child = driver()
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut next_id = 0u64;
    let mut call = |method: &str, params: Value| -> Value {
        next_id += 1;
        let id = next_id;
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
        )
        .unwrap();
        stdin.flush().unwrap();
        loop {
            let line = lines.next().unwrap().unwrap();
            let msg: Value = serde_json::from_str(&line).unwrap();
            if msg["id"] == json!(id) {
                return msg;
            }
        }
    };

    // initialize → tools/list: every replacement verb is a tool
    // (bare op name — the `cli.` prefix is stripped).
    call(
        "initialize",
        json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}),
    );
    let list = call("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for want in [
        "campaign.validate",
        "campaign.submit",
        "campaign.status",
        "campaign.events",
        "bundle.install",
        "bundle.remove",
        "bundle.list",
        "bundle.info",
        "logs.runs",
        "logs.events",
        "logs.explain",
    ] {
        assert!(
            names.contains(&want),
            "{want} missing from tools/list: {names:?}"
        );
    }
    // Unbounded surfaces never become tools.
    assert!(!names.contains(&"campaign.exec"));
    assert!(!names.contains(&"campaign.run"));

    // tools/call — read-only round-trips over the sandbox env.
    let result = call("tools/call", json!({"name":"bundle.list","arguments":{}}));
    assert_eq!(result["result"]["isError"], json!(false), "{result}");
    let text = result["result"]["content"][0]["text"].as_str().unwrap();
    let out: Value = serde_json::from_str(text).unwrap();
    assert!(out["bundles"].is_array(), "{out}");

    let result = call(
        "tools/call",
        json!({"name":"campaign.status","arguments":{"run":"no-such-run"}}),
    );
    assert_eq!(result["result"]["isError"], json!(false), "{result}");
    let text = result["result"]["content"][0]["text"].as_str().unwrap();
    let out: Value = serde_json::from_str(text).unwrap();
    assert_eq!(out["verdict"], "pending");

    // Bounded events tail through MCP — write a fixture log first.
    let log = roots().join("mcp-events.jsonl");
    write_log(
        &log,
        &[
            "{\"type\":\"n.run\",\"payload\":{},\"source\":\"n\",\"ts\":1}",
            "{\"type\":\"n.done\",\"payload\":{},\"source\":\"n\",\"ts\":2}",
        ],
    );
    let result = call(
        "tools/call",
        json!({"name":"logs.events","arguments":{"path":log.to_str().unwrap(),"limit":1}}),
    );
    assert_eq!(result["result"]["isError"], json!(false), "{result}");
    let text = result["result"]["content"][0]["text"].as_str().unwrap();
    let out: Value = serde_json::from_str(text).unwrap();
    assert_eq!(out["total"], 2);
    assert_eq!(out["events"].as_array().unwrap().len(), 1);

    child.kill().unwrap();
}
