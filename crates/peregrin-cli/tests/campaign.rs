#![allow(clippy::unwrap_used, clippy::pedantic)]

//! `peregrin campaign` verb coverage — run/validate/replay against
//! `peregrin_cli::campaign` plus a spawned-binary e2e proving the
//! `peregrin` CLI drives the campaign plane end-to-end.

use std::path::Path;
use std::path::PathBuf;

use peregrin_cli::campaign::ObsArgs;

/// The repo's `libraries/` dir — real loader input (includes,
/// `.node.yml` artifacts).
fn libraries_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../libraries")
        .canonicalize()
        .unwrap()
}

fn write_chain(dir: &Path) -> PathBuf {
    let yaml = format!(
        r#"name: e2e-chain
nodes:
  - id: write
    driver: shell
    on: [start]
    emits: [write.done, write.failed]
    params:
      cmd: "printf alpha > {dir}/alpha.txt"
      emit:
        "0": write.done
        default: write.failed
  - id: check
    driver: shell
    on: [write.done]
    emits: [check.done, check.failed]
    params:
      cmd: "grep -q alpha {dir}/alpha.txt"
      emit:
        "0": check.done
        default: check.failed
"#,
        dir = dir.display()
    );
    let path = dir.join("chain.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn run_executes_and_returns_the_log() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let log =
        peregrin_cli::campaign::run(&path, false, None, None, None, None, &ObsArgs::default())
            .unwrap();
    assert_eq!(log.last().map(|e| e.kind.as_str()), Some("workflow.done"));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("alpha.txt")).unwrap(),
        "alpha"
    );
}

#[test]
fn run_dry_run_emits_without_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let log = peregrin_cli::campaign::run(&path, true, None, None, None, None, &ObsArgs::default())
        .unwrap();
    assert_eq!(log.last().map(|e| e.kind.as_str()), Some("workflow.done"));
    assert!(!tmp.path().join("alpha.txt").exists());
}

#[test]
fn run_replay_seeds_from_jsonl_and_json() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    // JSONL: a `write.done` in history fires `check` (and exhausts
    // `write` via the seeded `start`).
    let jsonl = tmp.path().join("replay.jsonl");
    std::fs::write(
        &jsonl,
        "{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\"}\n\
         {\"type\":\"write.done\",\"payload\":{},\"source\":\"write\"}\n",
    )
    .unwrap();
    let log = peregrin_cli::campaign::run(
        &path,
        false,
        Some(&jsonl),
        None,
        None,
        None,
        &ObsArgs::default(),
    )
    .unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds.last(), Some(&"workflow.done"), "trace: {kinds:?}");
    // JSON array form parses too.
    let json = tmp.path().join("replay.json");
    std::fs::write(
        &json,
        "[{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\"}]",
    )
    .unwrap();
    let log = peregrin_cli::campaign::run(
        &path,
        false,
        Some(&json),
        None,
        None,
        None,
        &ObsArgs::default(),
    )
    .unwrap();
    assert!(!log.is_empty());
}

#[test]
fn run_replay_errors_on_missing_and_malformed() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    assert!(
        peregrin_cli::campaign::run(
            &path,
            false,
            Some(Path::new("/nonexistent.jsonl")),
            None,
            None,
            None,
            &ObsArgs::default()
        )
        .is_err()
    );
    let bad = tmp.path().join("bad.jsonl");
    std::fs::write(&bad, "{not json\n").unwrap();
    assert!(
        peregrin_cli::campaign::run(
            &path,
            false,
            Some(&bad),
            None,
            None,
            None,
            &ObsArgs::default()
        )
        .is_err()
    );
}

#[test]
fn run_errors_on_missing_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(
        peregrin_cli::campaign::run(
            &tmp.path().join("nope.yaml"),
            false,
            None,
            None,
            None,
            None,
            &ObsArgs::default()
        )
        .is_err()
    );
}

#[test]
fn validate_reports_wiring_and_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    // A node subscribing to an event nothing emits warns; the report
    // still ends OK (the TS behaviour — warnings, not failure).
    let yaml = tmp.path().join("warn.yaml");
    std::fs::write(
        &yaml,
        "name: warn-test\nnodes:\n  - id: orphan\n    driver: shell\n    on: [never.emitted]\n",
    )
    .unwrap();
    let lines = peregrin_cli::campaign::validate(&yaml).unwrap();
    assert_eq!(
        lines.first().map(String::as_str),
        Some("workflow: warn-test")
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("WARNING: node 'orphan' subscribes to 'never.emitted'")),
        "lines: {lines:?}"
    );
    assert_eq!(lines.last().map(String::as_str), Some("OK"));
}

#[test]
fn validate_accepts_a_wildcard_extension_of_a_declared_emit() {
    // `push.gated` is a templated emit's base — the engine appends the
    // captures at runtime — so a wildcard subscriber is wired, not an
    // orphan (the exact-match validator warned here).
    let tmp = tempfile::tempdir().unwrap();
    let yaml = tmp.path().join("wild.yaml");
    std::fs::write(
        &yaml,
        "name: wild\nnodes:\n  - id: emitter\n    driver: shell\n    on: [start]\n    emits: [push.gated]\n  - id: sub\n    driver: shell\n    on: [push.gated.*.**.*]\n",
    )
    .unwrap();
    let lines = peregrin_cli::campaign::validate(&yaml).unwrap();
    assert!(
        !lines.iter().any(|l| l.contains("WARNING")),
        "lines: {lines:?}"
    );
    assert_eq!(lines.last().map(String::as_str), Some("OK"));
}

#[test]
fn validate_a_libraries_campaign() {
    // Real stdlib yaml — includes/snippets expanded; every `on`
    // pattern is emitted, so no warnings.
    let lines =
        peregrin_cli::campaign::validate(&libraries_dir().join("meta/workflows/create/main.yaml"))
            .unwrap();
    assert!(lines.iter().any(|l| l == "workflow: meta-create"));
    assert!(
        !lines.iter().any(|l| l.contains("WARNING")),
        "lines: {lines:?}"
    );
    assert_eq!(lines.last().map(String::as_str), Some("OK"));
}

#[test]
fn validate_docs_deploy_bundle() {
    // PL-009: the docs-deploy bundle ships its own ingress node, so —
    // unlike an include-only main.yaml — it must validate with
    // zero unemitting-subscription warnings.
    let lines =
        peregrin_cli::campaign::validate(&libraries_dir().join("docs-deploy/main.yaml")).unwrap();
    assert!(lines.iter().any(|l| l == "workflow: docs-deploy"));
    assert!(
        !lines.iter().any(|l| l.contains("WARNING")),
        "lines: {lines:?}"
    );
    assert_eq!(lines.last().map(String::as_str), Some("OK"));
}

#[test]
fn validate_errors_on_missing_file() {
    assert!(peregrin_cli::campaign::validate(Path::new("/nonexistent.yaml")).is_err());
}

#[test]
fn serve_expands_dirs_and_errors_fast() {
    let tmp = tempfile::tempdir().unwrap();
    // `serve` blocks once campaigns boot — every assertion here must
    // fail *before* that point: path expansion, yaml load, or the
    // early `serve_campaigns` validation.
    std::fs::write(tmp.path().join("a.yaml"), "name: a\nnodes: []\n").unwrap();
    std::fs::write(tmp.path().join("b.yml"), "name: b\nnodes: []\n").unwrap();
    std::fs::write(tmp.path().join("skip.txt"), "name: x\n").unwrap();
    let rt = || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    };

    // A dir holding an unloadable yaml fails at load — proving the
    // *.yaml/*.yml glob expanded (skip.txt is never read).
    let bad = tmp.path().join("bad");
    std::fs::create_dir(&bad).unwrap();
    std::fs::write(bad.join("broken.yaml"), "nodes: [unclosed\n").unwrap();
    assert!(
        peregrin_cli::campaign::serve(
            &[bad],
            "127.0.0.1:0",
            "/hook",
            None,
            None,
            &ObsArgs::default(),
            &rt()
        )
        .is_err()
    );

    // Empty dir → no campaign yamls; missing path → does not exist.
    let empty = tmp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert!(
        peregrin_cli::campaign::serve(
            &[empty],
            "127.0.0.1:0",
            "/hook",
            None,
            None,
            &ObsArgs::default(),
            &rt()
        )
        .is_err()
    );
    assert!(
        peregrin_cli::campaign::serve(
            &[tmp.path().join("nope.yaml")],
            "127.0.0.1:0",
            "/hook",
            None,
            None,
            &ObsArgs::default(),
            &rt()
        )
        .is_err()
    );
}

#[test]
fn serve_expands_tilde_in_paths() {
    // Provider-rendered units carry `~` paths (ExecStart argv is data,
    // no shell) — `serve` must expand them before stat-ing. A missing
    // `~/…` file errors with the *expanded* absolute path.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let err = peregrin_cli::campaign::serve(
        &[PathBuf::from("~/peregrin-no-such-campaign-xyz.yaml")],
        "127.0.0.1:0",
        "/hook",
        None,
        None,
        &ObsArgs::default(),
        &rt,
    )
    .unwrap_err()
    .to_string();
    let home = std::env::var("HOME").unwrap();
    assert!(
        err.contains(&format!("{home}/peregrin-no-such-campaign-xyz.yaml")),
        "err: {err}"
    );
}

/// The binary e2e — the real engine drives load + engine + shell
/// side effects, and the event log lands on stdout. The `peregrin`
/// command is the tamias `bins[]` shim → `tamias-driver-peregrin`'s
/// engine passthrough, so the spawned surface under test is
/// `tamias-driver-peregrin run` (the same entrypoint the shim's
/// `cli` verb takes).
#[test]
fn peregrin_binary_runs_a_campaign() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let out =
        std::process::Command::new(sepia_testkit::ensure_driver_bin("tamias-driver-peregrin"))
            .args(["run", "campaign", "run", path.to_str().unwrap()])
            .output()
            .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("start"), "stdout: {stdout}");
    assert!(stdout.contains("write.done"), "stdout: {stdout}");
    assert!(stdout.contains("check.done"), "stdout: {stdout}");
    assert!(stdout.contains("workflow.done"), "stdout: {stdout}");
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("alpha.txt")).unwrap(),
        "alpha"
    );

    let out =
        std::process::Command::new(sepia_testkit::ensure_driver_bin("tamias-driver-peregrin"))
            .args([
                "run",
                "campaign",
                "validate",
                libraries_dir()
                    .join("meta/workflows/create/main.yaml")
                    .to_str()
                    .unwrap(),
            ])
            .output()
            .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("workflow: meta-create"), "stdout: {stdout}");
    assert!(stdout.contains("OK"), "stdout: {stdout}");
}

/// A one-node campaign whose shell exits 1 — the `emit` map routes it
/// onto `gate.failed`, so the run still ends `workflow.done` while
/// the verdict must read `fail`.
fn write_failing_gate(dir: &Path) -> PathBuf {
    let yaml = r#"name: failing-gate
nodes:
  - id: gate
    driver: shell
    on: [start]
    emits: [gate.passed, gate.failed]
    params:
      cmd: "false"
      emit:
        "0": gate.passed
        default: gate.failed
"#;
    let path = dir.join("failing-gate.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn run_writes_result_document_on_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let result_path = tmp.path().join("out").join("run.result.json");
    let log = peregrin_cli::campaign::run(
        &path,
        false,
        None,
        None,
        None,
        Some(&result_path),
        &ObsArgs::default(),
    )
    .unwrap();
    assert_eq!(log.last().map(|e| e.kind.as_str()), Some("workflow.done"));
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).unwrap();
    assert_eq!(doc["campaign"], "e2e-chain");
    assert_eq!(doc["verdict"], "pass");
    assert_eq!(doc["failed_events"], serde_json::json!([]));
    let nodes = doc["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["id"], "write");
    assert_eq!(nodes[0]["status"], "completed");
    assert_eq!(nodes[0]["rc"], 0);
    assert!(nodes[0]["secs"].is_number());
    assert!(nodes[0]["fires"].is_number());
    assert_eq!(nodes[1]["id"], "check");
    assert_eq!(nodes[1]["status"], "completed");
    assert_eq!(nodes[1]["conclusion"], "success");
}

#[test]
fn run_writes_fail_verdict_when_a_node_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_failing_gate(tmp.path());
    let result_path = tmp.path().join("gate.result.json");
    // The run still *completes* — `campaign run` returns Ok and the
    // process exit code stays 0; the artifact carries the verdict.
    let log = peregrin_cli::campaign::run(
        &path,
        false,
        None,
        None,
        None,
        Some(&result_path),
        &ObsArgs::default(),
    )
    .unwrap();
    assert_eq!(log.last().map(|e| e.kind.as_str()), Some("workflow.done"));
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).unwrap();
    assert_eq!(doc["verdict"], "fail");
    assert_eq!(doc["conclusion"], "failure");
    assert_eq!(doc["failed_events"], serde_json::json!(["gate.failed"]));
    let gate = &doc["nodes"][0];
    assert_eq!(gate["id"], "gate");
    assert_eq!(gate["status"], "completed");
    assert_eq!(gate["conclusion"], "failure");
    assert_eq!(gate["rc"], 1);
}

#[test]
fn run_history_implies_result_sibling() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let history = tmp.path().join("runs.jsonl");
    peregrin_cli::campaign::run(
        &path,
        false,
        None,
        Some(&history),
        None,
        None,
        &ObsArgs::default(),
    )
    .unwrap();
    // `<history>.result.json` — `with_extension` swaps `.jsonl`.
    let result_path = tmp.path().join("runs.result.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).unwrap();
    assert_eq!(doc["verdict"], "pass");
    assert!(history.exists(), "history log still written");
}

#[test]
fn run_result_path_beats_history_derivation() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let history = tmp.path().join("runs.jsonl");
    let explicit = tmp.path().join("explicit.json");
    peregrin_cli::campaign::run(
        &path,
        false,
        None,
        Some(&history),
        None,
        Some(&explicit),
        &ObsArgs::default(),
    )
    .unwrap();
    assert!(explicit.exists());
    assert!(
        !tmp.path().join("runs.result.json").exists(),
        "explicit --result wins over the history-derived sibling"
    );
}

#[test]
fn run_load_failure_writes_error_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("nope.yaml");
    let result_path = tmp.path().join("err.result.json");
    assert!(
        peregrin_cli::campaign::run(
            &missing,
            false,
            None,
            None,
            None,
            Some(&result_path),
            &ObsArgs::default()
        )
        .is_err()
    );
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).unwrap();
    assert_eq!(doc["verdict"], "error");
    assert_eq!(doc["conclusion"], "failure");
    assert_eq!(doc["campaign"], "nope");
}

/// A campaign whose `result:` block declares the verdict mapping —
/// the emitted `gate.pass` kind has no `.failed` segment, so without
/// the declaration the heuristic still passes; the block pins the
/// GH Checks vocabulary end-to-end.
#[test]
fn run_declared_result_block_maps_conclusion() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml = r#"name: declared-gate
result:
  - event: gate.pass
    conclusion: success
  - event: gate.fail
    conclusion: failure
nodes:
  - id: gate
    driver: shell
    on: [start]
    emits: [gate.pass]
    params:
      cmd: "true"
      emit:
        "0": gate.pass
"#;
    let path = tmp.path().join("declared-gate.yaml");
    std::fs::write(&path, yaml).unwrap();
    let result_path = tmp.path().join("declared.result.json");
    peregrin_cli::campaign::run(
        &path,
        false,
        None,
        None,
        None,
        Some(&result_path),
        &ObsArgs::default(),
    )
    .unwrap();
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).unwrap();
    assert_eq!(doc["schema"], "peregrin/run-result/1");
    assert_eq!(doc["status"], "completed");
    assert_eq!(doc["verdict"], "pass");
    assert_eq!(doc["conclusion"], "success");
    assert_eq!(doc["output"]["title"], "declared-gate");
    assert!(doc["run_id"].as_str().is_some_and(|id| id.len() == 16));
    assert!(doc["started_at"].is_string());
    assert!(doc["completed_at"].is_string());
}

/// `run_id` is stable for a run — projecting the same finished run's
/// log twice yields the same id (the engine test pins the
/// `from_log`/`run_result` equivalence; here the artifact itself is
/// the stable document).
#[test]
fn run_result_document_is_deterministic_in_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let first = tmp.path().join("first.json");
    let second = tmp.path().join("second.json");
    for out in [&first, &second] {
        peregrin_cli::campaign::run(
            &path,
            false,
            None,
            None,
            None,
            Some(out),
            &ObsArgs::default(),
        )
        .unwrap();
    }
    let a: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&first).unwrap()).unwrap();
    let b: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&second).unwrap()).unwrap();
    // Distinct runs → distinct run_ids; identical shape otherwise.
    assert_ne!(a["run_id"], b["run_id"]);
    for key in ["schema", "campaign", "status", "verdict", "conclusion"] {
        assert_eq!(a[key], b[key], "{key} drifted between runs");
    }
}
