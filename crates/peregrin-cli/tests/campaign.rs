#![allow(clippy::unwrap_used, clippy::pedantic)]

//! `peregrin campaign` verb coverage — run/validate/replay against
//! `peregrin_cli::campaign` plus a spawned-binary e2e proving the
//! `peregrin` CLI drives the campaign plane end-to-end.

use std::path::Path;
use std::path::PathBuf;

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
    let log = peregrin_cli::campaign::run(&path, false, None, None, None).unwrap();
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
    let log = peregrin_cli::campaign::run(&path, true, None, None, None).unwrap();
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
    let log = peregrin_cli::campaign::run(&path, false, Some(&jsonl), None, None).unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds.last(), Some(&"workflow.done"), "trace: {kinds:?}");
    // JSON array form parses too.
    let json = tmp.path().join("replay.json");
    std::fs::write(
        &json,
        "[{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\"}]",
    )
    .unwrap();
    let log = peregrin_cli::campaign::run(&path, false, Some(&json), None, None).unwrap();
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
            None
        )
        .is_err()
    );
    let bad = tmp.path().join("bad.jsonl");
    std::fs::write(&bad, "{not json\n").unwrap();
    assert!(peregrin_cli::campaign::run(&path, false, Some(&bad), None, None).is_err());
}

#[test]
fn run_errors_on_missing_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(
        peregrin_cli::campaign::run(&tmp.path().join("nope.yaml"), false, None, None, None)
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
    // unlike ci-gate's include-only main.yaml — it must validate with
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
        peregrin_cli::campaign::serve(&[bad], "127.0.0.1:0", "/hook", None, None, &rt()).is_err()
    );

    // Empty dir → no campaign yamls; missing path → does not exist.
    let empty = tmp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert!(
        peregrin_cli::campaign::serve(&[empty], "127.0.0.1:0", "/hook", None, None, &rt()).is_err()
    );
    assert!(
        peregrin_cli::campaign::serve(
            &[tmp.path().join("nope.yaml")],
            "127.0.0.1:0",
            "/hook",
            None,
            None,
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
