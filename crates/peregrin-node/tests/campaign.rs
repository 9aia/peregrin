#![allow(clippy::unwrap_used, clippy::pedantic)]

//! Campaign-plane e2e — the node's own composition
//! (`peregrin_node::campaign`) loads a campaign yaml and runs it on
//! the in-process engine: registry → LocalBus/MemoryStore/MemoryDlq →
//! Runner. Fixtures use the shell driver so real side effects prove
//! the run executed.

use std::path::Path;
use std::path::PathBuf;

use peregrin_engine::RunOptions;
use peregrin_node::campaign;
use serde_json::json;

/// The repo's `libraries/` dir — campaigns there are real loader
/// inputs (includes/snippets/.node.yml files).
fn libraries_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../libraries")
        .canonicalize()
        .unwrap()
}

/// A two-node shell chain: `write` touches a file on `start`, `check`
/// greps it on `write.done`. `{dir}` placeholders are substituted
/// with the tempdir path.
fn write_fixture(dir: &Path, extra_nodes: &str) -> PathBuf {
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
{extra_nodes}"#,
        dir = dir.display()
    );
    let path = dir.join("chain.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

const CHECK_NODE: &str = r#"  - id: check
    driver: shell
    on: [write.done]
    emits: [check.done, check.failed]
    params:
      cmd: "grep -q alpha {dir}/alpha.txt"
      emit:
        "0": check.done
        default: check.failed
"#;

#[test]
fn shell_chain_runs_to_workflow_done() {
    let tmp = tempfile::tempdir().unwrap();
    let check = CHECK_NODE.replace("{dir}", &tmp.path().display().to_string());
    let path = write_fixture(tmp.path(), &check);

    let log = campaign::run(&path, &RunOptions::default()).unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds.first(), Some(&"start"));
    assert!(kinds.contains(&"write.done"), "trace: {kinds:?}");
    assert!(kinds.contains(&"check.done"), "trace: {kinds:?}");
    assert_eq!(kinds.last(), Some(&"workflow.done"), "trace: {kinds:?}");
    // The shell side effect actually happened.
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("alpha.txt")).unwrap(),
        "alpha"
    );
}

#[test]
fn dry_run_skips_driver_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let check = CHECK_NODE.replace("{dir}", &tmp.path().display().to_string());
    let path = write_fixture(tmp.path(), &check);

    let log = campaign::run(
        &path,
        &RunOptions {
            dry_run: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds.last(), Some(&"workflow.done"), "trace: {kinds:?}");
    assert!(!tmp.path().join("alpha.txt").exists());
}

#[test]
fn registry_resolves_the_default_drivers() {
    let registry = campaign::registry();
    for name in ["agent", "http", "human", "shell", "noop", "runner"] {
        assert!(registry.resolve(name).is_ok(), "missing driver {name}");
    }
    // Namespaced resolution — `ns:x` → `ns`.
    assert!(registry.resolve("shell:whatever").is_ok());
    assert!(registry.resolve("unported-driver").is_err());
}

#[test]
fn load_expands_a_libraries_campaign() {
    // Real stdlib yaml: `include:` node files + snippet expansion must
    // produce a fully-wired Campaign.
    let campaign = campaign::load(&libraries_dir().join("common/new-feat/main.yaml")).unwrap();
    assert_eq!(campaign.name, "new-feat");
    assert!(
        campaign.nodes.len() >= 10,
        "expected include-expanded nodes, got {}",
        campaign.nodes.len()
    );
    assert!(
        campaign
            .nodes
            .iter()
            .any(|n| n.id == "new-feat.impl" && n.driver == "agent"),
        "agent node missing: {:?}",
        campaign.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
}

#[test]
fn load_errors_on_missing_and_malformed() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(campaign::load(&tmp.path().join("nope.yaml")).is_err());
    let bad = tmp.path().join("bad.yaml");
    std::fs::write(&bad, "- this is a list, not a campaign mapping\n").unwrap();
    assert!(campaign::load(&bad).is_err());
}

#[test]
fn runner_with_log_persists_the_event_log() {
    use peregrin_store::EventStore;
    let tmp = tempfile::tempdir().unwrap();
    let check = CHECK_NODE.replace("{dir}", &tmp.path().display().to_string());
    let yaml = write_fixture(tmp.path(), &check);
    let log_path = tmp.path().join("run.jsonl");

    let loaded = campaign::load(&yaml).unwrap();
    let mut runner = campaign::runner_with_log(&log_path).unwrap();
    let log = runner.run(&loaded, &RunOptions::default()).unwrap();
    assert_eq!(log.last().unwrap().kind, "workflow.done");

    // One durable JSON line per published event; a store reopened on
    // the same path resumes the run's publish + first-seen order.
    let store = peregrin_store_json::JsonStore::open(&log_path).unwrap();
    assert_eq!(store.all().len(), log.len());
    let mut expected = Vec::new();
    for e in &log {
        if !expected.contains(&e.kind) {
            expected.push(e.kind.clone());
        }
    }
    assert_eq!(store.seen_types(), expected);
    assert_eq!(store.latest().unwrap().kind, "workflow.done");
}

#[test]
fn runner_with_log_errors_on_a_malformed_log() {
    let tmp = tempfile::tempdir().unwrap();
    let log_path = tmp.path().join("run.jsonl");
    std::fs::write(&log_path, "not json\n").unwrap();
    let err = campaign::runner_with_log(&log_path).err().unwrap();
    assert!(err.to_string().contains("line 1"), "{err}");
}

#[test]
fn run_surfaces_engine_failures() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml = tmp.path().join("unknown-driver.yaml");
    std::fs::write(
        &yaml,
        "nodes:\n  - id: x\n    driver: no-such-driver\n    on: [start]\n",
    )
    .unwrap();
    let err = campaign::run(&yaml, &RunOptions::default()).unwrap_err();
    assert!(err.to_string().contains("no-such-driver"), "{err}");
}

/// Agentic orchestration e2e — `mock-agent` nodes chain like real
/// agent nodes: `plan` emits `agent.done`, `execute` fires on it
/// with a prompt templated from the plan's payload, a `verify` node
/// joins on `execute.done`. The event log is the assertion: every
/// hop's payload flows through the graph.
#[test]
fn mock_agent_chain_orchestrates_plan_execute_verify() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml = tmp.path().join("agentic.yaml");
    std::fs::write(
        &yaml,
        r#"name: agentic
nodes:
  - id: plan
    driver: mock-agent
    on: [start]
    emits: [agent.done]
    params:
      prompt: "draft the plan"
      response: "plan: migrate ci"

  - id: execute
    driver: mock-agent
    on: [agent.done]
    emits: [execute.done]
    params:
      prompt: "run ${event.payload.stdout}"
      response: "applied"
      emit: {}
"#,
    )
    .unwrap();
    let log = campaign::run(&yaml, &RunOptions::default()).unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"agent.done"), "{kinds:?}");
    let exec = log
        .iter()
        .find(|e| e.kind == "agent.done" || e.payload["runner"] == "mock")
        .unwrap();
    // The second agent.done carries the templated prompt — proving
    // the upstream payload reached the next node's params.
    let agent_events: Vec<_> = log.iter().filter(|e| e.kind == "agent.done").collect();
    assert_eq!(agent_events.len(), 2, "{kinds:?}");
    assert_eq!(agent_events[1].payload["prompt"], "run plan: migrate ci");
    let _ = exec;
    assert_eq!(kinds.last(), Some(&"workflow.done"), "{kinds:?}");
}

/// Failure propagates the same way — a scripted `fail` turns into
/// `node.failed`, which a downstream compensator can subscribe to.
#[test]
fn mock_agent_failure_routes_to_compensator() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml = tmp.path().join("agentic-fail.yaml");
    std::fs::write(
        &yaml,
        r#"name: agentic-fail
nodes:
  - id: attempt
    driver: mock-agent
    on: [start]
    params:
      prompt: "try the thing"
      fail: true
      fail_message: "quota exhausted"

  - id: compensate
    driver: noop
    on: [node.failed]
"#,
    )
    .unwrap();
    let log = campaign::run(&yaml, &RunOptions::default()).unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    let failure = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(failure.payload["error"], "quota exhausted");
    assert!(kinds.contains(&"node.done"), "{kinds:?}");
}

// --- secret resolution chain (I-018) ----------------------------------------

/// `campaign::run` wires the default fallback stores: OS keyring
/// declared-only, then the env file — in that order, so the
/// documented `record → env → keyring → file` chain holds.
#[test]
fn default_secret_stores_wires_keyring_then_env_file() {
    let stores = campaign::default_secret_stores();
    assert_eq!(stores.len(), 2);
    assert!(stores[0].declared_only);
    assert!(!stores[1].declared_only);
}

/// The I-018 gap, end to end: a name absent from the env file and
/// the record resolves through the ambient environment leg inside
/// `campaign::run` — `export FOO=…` reaches `${secret.FOO}`.
#[test]
fn run_resolves_secrets_from_ambient_env() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml = tmp.path().join("secret-env.yaml");
    std::fs::write(
        &yaml,
        r#"name: secret-env
capabilities:
  secrets: true
nodes:
  - id: write
    driver: shell
    on: [start]
    emits: [write.done, write.failed]
    params:
      cmd: "echo ${secret.PATH}"
      emit:
        "0": write.done
        default: write.failed
"#,
    )
    .unwrap();
    let log = campaign::run(&yaml, &RunOptions::default()).unwrap();
    let done = log.iter().find(|e| e.kind == "write.done").unwrap();
    // The resolved cmd rides the emitted payload — PATH arrived via
    // the ambient-env leg, not the record or a store.
    assert_eq!(
        done.payload["cmd"],
        json!(format!("echo {}", std::env::var("PATH").unwrap()))
    );
}

/// A miss through every leg still resolves to `""` non-strict — the
/// shell node runs (and the run completes) rather than erroring.
#[test]
fn run_tolerates_a_fully_missing_secret_non_strict() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml = tmp.path().join("secret-miss.yaml");
    std::fs::write(
        &yaml,
        r#"name: secret-miss
capabilities:
  secrets: true
nodes:
  - id: write
    driver: shell
    on: [start]
    emits: [write.done, write.failed]
    params:
      cmd: "true"
      emit:
        "0": write.done
        default: write.failed
      probe: "${secret.PEREGRIN_TEST_DEFINITELY_UNSET_018}"
"#,
    )
    .unwrap();
    let log = campaign::run(&yaml, &RunOptions::default()).unwrap();
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}
