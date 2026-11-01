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
