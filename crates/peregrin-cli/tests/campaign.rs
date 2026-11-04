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
    let log = peregrin_cli::campaign::run(&path, false, None, None).unwrap();
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
    let log = peregrin_cli::campaign::run(&path, true, None, None).unwrap();
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
    let log = peregrin_cli::campaign::run(&path, false, Some(&jsonl), None).unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds.last(), Some(&"workflow.done"), "trace: {kinds:?}");
    // JSON array form parses too.
    let json = tmp.path().join("replay.json");
    std::fs::write(
        &json,
        "[{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\"}]",
    )
    .unwrap();
    let log = peregrin_cli::campaign::run(&path, false, Some(&json), None).unwrap();
    assert!(!log.is_empty());
}

#[test]
fn run_replay_errors_on_missing_and_malformed() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    assert!(
        peregrin_cli::campaign::run(&path, false, Some(Path::new("/nonexistent.jsonl")), None)
            .is_err()
    );
    let bad = tmp.path().join("bad.jsonl");
    std::fs::write(&bad, "{not json\n").unwrap();
    assert!(peregrin_cli::campaign::run(&path, false, Some(&bad), None).is_err());
}

#[test]
fn run_errors_on_missing_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(peregrin_cli::campaign::run(&tmp.path().join("nope.yaml"), false, None, None).is_err());
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
fn validate_a_libraries_campaign() {
    // Real stdlib yaml — includes/snippets expanded; every `on`
    // pattern is emitted, so no warnings.
    let lines =
        peregrin_cli::campaign::validate(&libraries_dir().join("common/new-feat/main.yaml"))
            .unwrap();
    assert!(lines.iter().any(|l| l == "workflow: new-feat"));
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

/// The binary e2e — the real `peregrin` drives load + engine + shell
/// side effects, and the event log lands on stdout.
#[test]
fn peregrin_binary_runs_a_campaign() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_chain(tmp.path());
    let out = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin"))
        .args(["campaign", "run", path.to_str().unwrap()])
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

    let out = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin"))
        .args([
            "campaign",
            "validate",
            libraries_dir()
                .join("common/new-feat/main.yaml")
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
    assert!(stdout.contains("workflow: new-feat"), "stdout: {stdout}");
    assert!(stdout.contains("OK"), "stdout: {stdout}");
}
