//! Port of `ac-loader-yaml/tests/index.test.ts` — same cases against
//! `YamlLoader`, plus cycle/error-position checks and a real
//! `libraries/` fixture parse.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;

fn loader() -> YamlLoader {
    YamlLoader
}

fn load(text: &str, dir: &Path) -> Result<peregrin_core::Campaign, peregrin_loader::LoadError> {
    loader().load_str(text, dir)
}

#[test]
fn loads_a_minimal_workflow_from_text() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load("name: test\nnodes: []", tmp.path()).unwrap();
    assert_eq!(campaign.name, "test");
    assert_eq!(campaign.nodes.len(), 0);
}

#[test]
fn fails_when_nodes_is_not_an_array() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load("name: test\nnodes: notArray", tmp.path()).unwrap_err();
    assert_eq!(err.reason(), "'nodes' must be an array");
}

#[test]
fn fails_when_workflow_is_not_a_mapping() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load("- a\n- b", tmp.path()).unwrap_err();
    assert_eq!(err.reason(), "workflow must be a mapping");
}

#[test]
fn merges_include_into_node_and_local_wins() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("env.yaml"),
        "env:\n  REGION: us-east-1\ntimeout: 300\n",
    )
    .unwrap();
    let campaign = load(
        "name: test\nnodes:\n  - id: deploy\n    driver: shell\n    on: [start]\n    emits: [deploy.done]\n    params:\n      include: env.yaml\n      cmd: deploy.sh",
        tmp.path(),
    )
    .unwrap();
    let params = &campaign.nodes[0].params;
    assert_eq!(params["env"], serde_json::json!({"REGION": "us-east-1"}));
    assert_eq!(params["timeout"], 300);
    assert_eq!(params["cmd"], "deploy.sh");
    assert!(params.get("include").is_none());
}

#[test]
fn injects_a_whole_dict_via_include_expression() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("env.yaml"),
        "REGION: us-east-1\nCLUSTER: prod\n",
    )
    .unwrap();
    let campaign = load(
        "name: test\nnodes:\n  - id: deploy\n    driver: shell\n    on: [start]\n    params:\n      env: ${include(env.yaml)}\n      cmd: deploy.sh",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(
        campaign.nodes[0].params["env"],
        serde_json::json!({"REGION": "us-east-1", "CLUSTER": "prod"})
    );
}

#[test]
fn injects_a_whole_list_via_include_expression() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("shards.yaml"),
        "- id: us\n  driver: shell\n  on: [start]\n  emits: [us.done]\n- id: eu\n  driver: shell\n  on: [start]\n  emits: [eu.done]\n",
    )
    .unwrap();
    let campaign = load("name: test\nnodes: ${include(shards.yaml)}", tmp.path()).unwrap();
    assert_eq!(campaign.nodes.len(), 2);
    assert_eq!(campaign.nodes[0].id, "us");
    assert_eq!(campaign.nodes[1].id, "eu");
}

#[test]
fn injects_scalar_and_partial_include_expressions() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("version.yaml"), "1.2.3").unwrap();
    let campaign = load(
        "name: deploy-${include(version.yaml)}\nnodes:\n  - id: deploy\n    driver: shell\n    on: [start]\n    params:\n      cmd: echo version ${include(version.yaml)}",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.name, "deploy-1.2.3");
    assert_eq!(campaign.nodes[0].params["cmd"], "echo version 1.2.3");
}

#[test]
fn uses_first_include_when_include_is_a_list() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("a.yaml"),
        "id: node-a\ndriver: shell\non: [start]\nemits: [a.done]\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("b.yaml"),
        "id: node-b\non: [a.done]\nemits: [b.done]\n",
    )
    .unwrap();
    let campaign = load(
        "name: test\nnodes:\n  - include: [a.yaml, b.yaml]",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes.len(), 1);
    assert_eq!(campaign.nodes[0].id, "node-a");
}

#[test]
fn recursively_merges_include_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("base.yaml"), "env:\n  PYTHONPATH: /app\n").unwrap();
    fs::write(
        tmp.path().join("prod.yaml"),
        "include: base.yaml\nenv:\n  REGION: us-east-1\n",
    )
    .unwrap();
    let campaign = load(
        "name: test\nnodes:\n  - id: deploy\n    driver: shell\n    on: [start]\n    params:\n      include: prod.yaml",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(
        campaign.nodes[0].params["env"],
        serde_json::json!({"PYTHONPATH": "/app", "REGION": "us-east-1"})
    );
}

#[test]
fn resolves_recursive_include_expressions() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("base.yaml"), "PYTHONPATH: /app\n").unwrap();
    fs::write(
        tmp.path().join("prod.yaml"),
        "include: base.yaml\nREGION: us-east-1\n",
    )
    .unwrap();
    let campaign = load(
        "name: test\nnodes:\n  - id: deploy\n    driver: shell\n    on: [start]\n    params:\n      env: ${include(prod.yaml)}",
        tmp.path(),
    )
    .unwrap();
    let env = &campaign.nodes[0].params["env"];
    assert_eq!(env["PYTHONPATH"], "/app");
    assert_eq!(env["REGION"], "us-east-1");
}

#[test]
fn detects_circular_includes_by_key() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.yaml"), "include: b.yaml\nvalue: a\n").unwrap();
    fs::write(tmp.path().join("b.yaml"), "include: a.yaml\nvalue: b\n").unwrap();
    let err = loader()
        .load_raw("include: a.yaml", tmp.path())
        .unwrap_err();
    assert!(err.reason().starts_with("circular include detected:"));
}

#[test]
fn detects_circular_includes_by_expression() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("a.yaml"), "value: ${include(b.yaml)}\n").unwrap();
    fs::write(tmp.path().join("b.yaml"), "value: ${include(a.yaml)}\n").unwrap();
    let err = loader()
        .load_raw(
            "nodes:\n  - id: x\n    params:\n      val: ${include(a.yaml)}",
            tmp.path(),
        )
        .unwrap_err();
    assert!(err.reason().starts_with("circular include detected:"));
}

#[test]
fn detects_self_includes() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("main.yaml"), "include: main.yaml\n").unwrap();
    let err = loader()
        .load_raw("include: main.yaml", tmp.path())
        .unwrap_err();
    assert!(err.reason().starts_with("circular include detected:"));
}

#[test]
fn raises_for_missing_include_files() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load(
        "nodes:\n  - id: x\n    params:\n      include: missing.yaml",
        tmp.path(),
    )
    .unwrap_err();
    assert!(err.reason().starts_with("Failed to read file:"));
}

#[test]
fn rejects_non_string_include_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let err = loader()
        .load_raw("include: [a.yaml, 5]", tmp.path())
        .unwrap_err();
    // a.yaml is missing → read error fires first, matching TS order.
    assert!(err.reason().starts_with("Failed to read file:"));
    fs::write(tmp.path().join("a.yaml"), "x: 1\n").unwrap();
    let err = loader()
        .load_raw("include: [a.yaml, 5]", tmp.path())
        .unwrap_err();
    assert_eq!(err.reason(), "include path must be a string");
}

#[test]
fn merges_a_snippet_into_a_node() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load(
        "name: test\nsnippets:\n  base:\n    driver: shell\n    emits: [base.done]\n    params:\n      timeout: 600\n      env:\n        A: 1\nnodes:\n  - id: n\n    snippet: base\n    on: [start]\n    params:\n      env:\n        B: 2",
        tmp.path(),
    )
    .unwrap();
    let node = &campaign.nodes[0];
    assert_eq!(node.driver, "shell");
    assert_eq!(node.emits, ["base.done"]);
    assert_eq!(node.params["timeout"], 600);
    assert_eq!(node.params["env"], serde_json::json!({"A": 1, "B": 2}));
}

#[test]
fn injects_a_scalar_snippet_via_expression() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load(
        "name: test\nsnippets:\n  cmd: echo hello\nnodes:\n  - id: n\n    driver: shell\n    on: [start]\n    params:\n      cmd: ${snippet(cmd)}",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes[0].params["cmd"], "echo hello");
}

#[test]
fn injects_a_list_snippet_via_expression() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load(
        "name: test\nsnippets:\n  triggers: [pr.opened, pr.updated]\nnodes:\n  - id: n\n    driver: shell\n    on: ${snippet(triggers)}\n    params:\n      cmd: echo ok",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes[0].on, ["pr.opened", "pr.updated"]);
}

#[test]
fn injects_a_dict_snippet_via_expression() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load(
        "name: test\nsnippets:\n  env:\n    REGION: us-east-1\n    CLUSTER: prod\nnodes:\n  - id: n\n    driver: shell\n    on: [start]\n    params:\n      env: ${snippet(env)}\n      cmd: echo ok",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(
        campaign.nodes[0].params["env"],
        serde_json::json!({"REGION": "us-east-1", "CLUSTER": "prod"})
    );
}

#[test]
fn interpolates_a_snippet_into_a_partial_string() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load(
        "name: test\nsnippets:\n  model: sonnet\nnodes:\n  - id: n\n    driver: shell\n    on: [start]\n    params:\n      cmd: run --model ${snippet(model)}",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes[0].params["cmd"], "run --model sonnet");
}

#[test]
fn raises_for_unknown_snippet_references() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load(
        "name: test\nnodes:\n  - id: n\n    snippet: missing\n    on: [start]",
        tmp.path(),
    )
    .unwrap_err();
    assert_eq!(err.reason(), "unknown snippet 'missing'");
}

#[test]
fn resolves_three_levels_of_snippet_inheritance() {
    let tmp = tempfile::tempdir().unwrap();
    let campaign = load(
        "name: test\nsnippets:\n  a:\n    params:\n      x: 1\n  b:\n    snippet: a\n    params:\n      y: 2\n  c:\n    snippet: b\n    params:\n      z: 3\nnodes:\n  - id: n\n    snippet: c\n    on: [start]",
        tmp.path(),
    )
    .unwrap();
    let params = &campaign.nodes[0].params;
    assert_eq!(params["x"], 1);
    assert_eq!(params["y"], 2);
    assert_eq!(params["z"], 3);
}

#[test]
fn detects_self_referencing_snippet_inheritance() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load(
        "name: test\nsnippets:\n  loop:\n    snippet: loop\nnodes:\n  - id: n\n    snippet: loop\n    on: [start]",
        tmp.path(),
    )
    .unwrap_err();
    assert_eq!(
        err.reason(),
        "circular snippet: 'loop' inherits from itself"
    );
}

#[test]
fn rejects_non_dict_snippets_table() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load("name: test\nsnippets: [a, b]\nnodes: []", tmp.path()).unwrap_err();
    assert_eq!(err.reason(), "'snippets' must be a mapping");
}

#[test]
fn rejects_removed_with_field() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load(
        "name: test\nnodes:\n  - id: n\n    driver: shell\n    with:\n      cmd: echo hi",
        tmp.path(),
    )
    .unwrap_err();
    assert_eq!(
        err.reason(),
        "Node 'n' uses removed 'with' field; use 'params' instead"
    );
}

#[test]
fn auto_discovers_nodes_and_snippets_from_the_handl_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let ac_dir = tmp.path().join(".handl");
    fs::create_dir(&ac_dir).unwrap();
    fs::write(
        ac_dir.join("shared.snippet.yml"),
        "driver: shell\nparams:\n  timeout: 1200",
    )
    .unwrap();
    fs::write(
        ac_dir.join("auto.node.yml"),
        "id: auto\ndriver: shell\non: [start]",
    )
    .unwrap();
    let campaign = load(
        "name: test\nnodes:\n  - id: n\n    snippet: shared\n    on: [start]",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes.len(), 2);
    assert_eq!(campaign.nodes[0].id, "auto");
    assert_eq!(campaign.nodes[1].id, "n");
    assert_eq!(campaign.nodes[1].params["timeout"], 1200);
}

#[test]
fn auto_discovers_nodes_and_snippets_from_the_peregrin_directory() {
    // `.peregrin/` is the rebrand discovery root — same scan as
    // `.handl/`, preferred on dedup (scanned first).
    let tmp = tempfile::tempdir().unwrap();
    let pg_dir = tmp.path().join(".peregrin");
    fs::create_dir(&pg_dir).unwrap();
    fs::write(
        pg_dir.join("shared.snippet.yml"),
        "driver: shell
params:
  timeout: 900",
    )
    .unwrap();
    fs::write(
        pg_dir.join("auto.node.yml"),
        "id: auto
driver: shell
on: [start]",
    )
    .unwrap();
    let campaign = load(
        "name: test
nodes:
  - id: n
    snippet: shared
    on: [start]",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes.len(), 2);
    assert_eq!(campaign.nodes[0].id, "auto");
    assert_eq!(campaign.nodes[1].params["timeout"], 900);
}

#[test]
fn peregrin_root_wins_dedup_over_handl() {
    // Both roots present: `.peregrin` is scanned first, so its entries
    // claim shared ids/names.
    let tmp = tempfile::tempdir().unwrap();
    for (dir, timeout) in [(".peregrin", 100), (".handl", 200)] {
        let d = tmp.path().join(dir);
        fs::create_dir(&d).unwrap();
        fs::write(
            d.join("shared.snippet.yml"),
            format!(
                "driver: shell
params:
  timeout: {timeout}"
            ),
        )
        .unwrap();
    }
    let campaign = load(
        "name: test
nodes:
  - id: n
    snippet: shared
    on: [start]",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(campaign.nodes[0].params["timeout"], 100);
}

#[test]
fn yaml_parse_errors_carry_position() {
    let tmp = tempfile::tempdir().unwrap();
    let err = load("name: ok\nnodes:\n  - id: [", tmp.path()).unwrap_err();
    assert_eq!(err.reason(), "Failed to parse YAML");
    let source = std::error::Error::source(&err).unwrap().to_string();
    assert!(source.contains("line"), "expected position in: {source}");
}

// --- Real stdlib fixture ---------------------------------------------------

fn libraries_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries")
}

#[test]
fn loads_the_meta_create_stdlib_campaign_with_prompt_includes() {
    let campaign = loader()
        .load(&libraries_dir().join("meta/workflows/create/main.yaml"))
        .unwrap();
    assert_eq!(campaign.name, "meta-create");
    // 7 include-expanded nodes; snippet files fill the body, the
    // parent overrides `on`/`params` locally.
    assert_eq!(campaign.nodes.len(), 7);
    assert_eq!(campaign.nodes[0].id, "agent.generate");
    assert_eq!(campaign.nodes[0].driver, "agent");
    // ${include(../../generate.prompt.yml)} expanded into the prompt param.
    let prompt = campaign.nodes[0].params["prompt"].as_str().unwrap();
    assert!(prompt.contains("${WORKFLOW_PATH}"), "prompt: {prompt}");
    // Numeric `emit:` keys coerced to strings like the JS `yaml` output.
    assert_eq!(campaign.nodes[0].params["emit"]["0"], "agent.generate.done");
    assert_eq!(
        campaign.nodes[0].params["emit"]["default"],
        "agent.generate.failed"
    );
}

#[test]
fn loads_the_meta_create_stdlib_campaign() {
    let campaign = loader()
        .load(&libraries_dir().join("meta/workflows/create/main.yaml"))
        .unwrap();
    assert_eq!(campaign.name, "meta-create");
    assert_eq!(campaign.nodes.len(), 7);
    // `on:` overrides locally; snippet-style includes fill the rest.
    assert_eq!(
        campaign.nodes[1].on,
        ["agent.generate.done", "agent.fix.done"]
    );
    assert_eq!(campaign.events[1].fields["stdout"], "string");
}

// --- Node-id dedupe (double-fire guard) -------------------------------------

#[test]
fn include_redefining_a_node_id_merges_not_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("base.yaml"),
        "nodes:\n  - id: gate\n    driver: shell\n    on: [start]\n    params:\n      cmd: base\n  - id: base-only\n    driver: shell\n",
    )
    .unwrap();
    let campaign = load(
        "name: over\ninclude: base.yaml\nnodes:\n  - id: gate\n    driver: agent\n    on: [go]\n  - id: local-only\n    driver: shell\n",
        tmp.path(),
    )
    .unwrap();
    // `gate` kept once, from the local definition; base-only survives.
    let ids: Vec<&str> = campaign.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["gate", "local-only", "base-only"]);
    assert_eq!(campaign.nodes[0].driver, "agent");
    assert_eq!(campaign.nodes[0].on, ["go"]);
}

#[test]
fn dedupe_helpers_cover_edges() {
    use peregrin_loader_yaml::{dedupe_node_ids, deep_merge, merge_documents};
    use serde_json::{Map, Value, json};

    // dedupe_node_ids: no nodes key / non-array / non-string ids pass.
    let mut none: Map<String, Value> = Map::new();
    dedupe_node_ids(&mut none);
    let mut scalar = Map::from_iter([("nodes".to_string(), json!("x"))]);
    dedupe_node_ids(&mut scalar);
    assert_eq!(scalar["nodes"], json!("x"));
    let mut odd = Map::from_iter([(
        "nodes".to_string(),
        json!([{"driver": "shell"}, {"id": 3}, {"id": "a"}, {"id": "a"}, "plain"]),
    )]);
    dedupe_node_ids(&mut odd);
    assert_eq!(
        odd["nodes"],
        json!([{"driver": "shell"}, {"id": 3}, {"id": "a"}, "plain"])
    );

    // merge_documents: deep_merge shape + dedupe.
    let local = json!({"nodes": [{"id": "n", "params": {"a": 1}}], "x": [1]});
    let imported = json!({"nodes": [{"id": "n", "params": {"b": 2}}, {"id": "m"}], "x": [2]});
    let merged = merge_documents(&local, &imported);
    assert_eq!(
        merged,
        json!({"nodes": [{"id": "n", "params": {"a": 1}}, {"id": "m"}], "x": [1, 2]})
    );
    // Non-object merge result skips the dedupe branch.
    assert_eq!(merge_documents(&json!(1), &json!({"a": 2})), json!(1));
    // deep_merge stays reachable through the re-export.
    assert_eq!(
        deep_merge(&json!({"k": "v"}), &Value::Null),
        json!({"k": "v"})
    );
}

// --- params.schema.json (ADR-0081 / I-022) ---------------------------------

const PARAMS_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": ["token_env"],
  "properties": {
    "token_env": {"type": "string"},
    "timeout": {"type": "integer", "minimum": 1}
  }
}"#;

#[test]
fn params_schema_accepts_a_valid_config() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("params.schema.json"), PARAMS_SCHEMA).unwrap();
    let campaign = load(
        "name: test\nconfig:\n  token_env: BUPHA_TOKEN\n  timeout: 60\nnodes: []",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(
        campaign.config.get("token_env").and_then(|v| v.as_str()),
        Some("BUPHA_TOKEN")
    );
}

#[test]
fn params_schema_rejects_a_typoed_key() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("params.schema.json"), PARAMS_SCHEMA).unwrap();
    let err = load(
        "name: test\nconfig:\n  token_env: BUPHA_TOKEN\n  timout: 60\nnodes: []",
        tmp.path(),
    )
    .unwrap_err();
    assert!(err.reason().contains("params.schema.json"), "{err}");
    assert!(err.reason().contains("timout"), "{err}");
}

#[test]
fn params_schema_rejects_missing_required_and_wrong_type() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("params.schema.json"), PARAMS_SCHEMA).unwrap();
    let err = load("name: test\nconfig:\n  timeout: 60\nnodes: []", tmp.path()).unwrap_err();
    assert!(err.reason().contains("token_env"), "{err}");
    let err = load(
        "name: test\nconfig:\n  token_env: BUPHA_TOKEN\n  timeout: fast\nnodes: []",
        tmp.path(),
    )
    .unwrap_err();
    assert!(err.reason().contains("timeout"), "{err}");
}

#[test]
fn absent_params_schema_leaves_config_unchecked() {
    let tmp = tempfile::tempdir().unwrap();
    // No params.schema.json — arbitrary config keys pass (plain
    // campaigns are not bundles).
    load(
        "name: test\nconfig:\n  anything: goes\nnodes: []",
        tmp.path(),
    )
    .unwrap();
}

// --- tamias: include refs ----------------------------------------------------
//
// `tamias:<name>[@<label>]//<path>` resolves against the tamias install
// store. Tests hand-write store entries (`.tamias-store.json` records —
// the same on-disk shape `tamias install` commits) into a tempdir and
// load through `load_str_with_tamias_store` — edition-2024 forbids
// `env::set_var`, so the env chain can't be exercised here.

/// Write a fake store entry — dir `<store>/<dir_name>/` carrying
/// `files` plus a minimal `.tamias-store.json` record.
fn fake_store_entry(
    store: &Path,
    dir_name: &str,
    name: &str,
    version: Option<&str>,
    installed_at: u64,
    files: &[(&str, &str)],
) {
    let entry = store.join(dir_name);
    fs::create_dir_all(&entry).unwrap();
    for (rel, text) in files {
        let path = entry.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let version_json = version.map_or_else(|| "null".to_string(), |v| format!("\"{v}\""));
    fs::write(
        entry.join(".tamias-store.json"),
        format!(
            "{{\"name\":\"{name}\",\"version\":{version_json},\"source_ref\":\"./x\",\
             \"resolved\":\"sha256:abc\",\"digest\":\"sha256:def\",\"installed_at\":{installed_at}}}"
        ),
    )
    .unwrap();
}

fn load_with_store(
    text: &str,
    dir: &Path,
    store: &Path,
) -> Result<peregrin_core::Campaign, peregrin_loader::LoadError> {
    YamlLoader.load_str_with_tamias_store(text, dir, store)
}

#[test]
fn tamias_include_merges_a_bundle_fragment() {
    let tmp = tempfile::tempdir().unwrap();
    // The store must sit outside `base_dir` — `.handl/` discovery scans
    // the workdir recursively and would eat `*.node.yml` payload files.
    let store_tmp = tempfile::tempdir().unwrap();
    let store = store_tmp.path().join("store");
    fake_store_entry(
        &store,
        "handl-gates@0.2.0",
        "handl-gates",
        Some("0.2.0"),
        1_700_000_000,
        &[("lib/rust-gate.node.yml", "timeout: 42\n")],
    );
    let campaign = load_with_store(
        "name: test\nnodes:\n  - id: gate\n    on: [start]\n    params:\n      include: tamias:handl-gates//lib/rust-gate.node.yml\n      cmd: gate.sh",
        tmp.path(),
        &store,
    )
    .unwrap();
    let params = &campaign.nodes[0].params;
    assert_eq!(params["timeout"], 42);
    assert_eq!(params["cmd"], "gate.sh");
    assert!(params.get("include").is_none());
}

#[test]
fn tamias_include_version_pin_and_latest_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path().join("store");
    // 0.1.0 installed *after* 0.2.0 — an unpinned ref takes the most
    // recently installed entry, a pin takes the named label.
    fake_store_entry(
        &store,
        "handl-gates@0.2.0",
        "handl-gates",
        Some("0.2.0"),
        1_700_000_000,
        &[("frag.yaml", "timeout: 200\n")],
    );
    fake_store_entry(
        &store,
        "handl-gates@0.1.0",
        "handl-gates",
        Some("0.1.0"),
        1_700_000_100,
        &[("frag.yaml", "timeout: 100\n")],
    );
    let pinned = load_with_store(
        "name: test\nnodes:\n  - id: n\n    on: [start]\n    params:\n      include: tamias:handl-gates@0.2.0//frag.yaml",
        tmp.path(),
        &store,
    )
    .unwrap();
    assert_eq!(pinned.nodes[0].params["timeout"], 200);
    let latest = load_with_store(
        "name: test\nnodes:\n  - id: n\n    on: [start]\n    params:\n      include: tamias:handl-gates//frag.yaml",
        tmp.path(),
        &store,
    )
    .unwrap();
    assert_eq!(latest.nodes[0].params["timeout"], 100);
}

#[test]
fn tamias_include_missing_bundle_names_tamias_install() {
    let tmp = tempfile::tempdir().unwrap();
    // The store must sit outside `base_dir` — `.handl/` discovery scans
    // the workdir recursively and would eat `*.node.yml` payload files.
    let store_tmp = tempfile::tempdir().unwrap();
    let store = store_tmp.path().join("store");
    fake_store_entry(
        &store,
        "handl-gates@0.2.0",
        "handl-gates",
        Some("0.2.0"),
        1_700_000_000,
        &[("frag.yaml", "timeout: 1\n")],
    );
    let err = load_with_store(
        "name: test\nnodes:\n  - id: n\n    on: [start]\n    params:\n      include: tamias:ghost//frag.yaml",
        tmp.path(),
        &store,
    )
    .unwrap_err();
    assert!(err.reason().contains("'ghost' is not installed"), "{err}");
    assert!(err.reason().contains("tamias install"), "{err}");
    // A missing *file* inside an installed bundle is a plain read error.
    let err = load_with_store(
        "name: test\nnodes:\n  - id: n\n    on: [start]\n    params:\n      include: tamias:handl-gates//missing.yaml",
        tmp.path(),
        &store,
    )
    .unwrap_err();
    assert!(err.reason().starts_with("Failed to read file:"), "{err}");
}

#[test]
fn tamias_include_expression_and_escape_guard() {
    let tmp = tempfile::tempdir().unwrap();
    // The store must sit outside `base_dir` — `.handl/` discovery scans
    // the workdir recursively and would eat `*.node.yml` payload files.
    let store_tmp = tempfile::tempdir().unwrap();
    let store = store_tmp.path().join("store");
    fake_store_entry(
        &store,
        "handl-gates@0.2.0",
        "handl-gates",
        Some("0.2.0"),
        1_700_000_000,
        &[("env.yaml", "REGION: us-east-1\n")],
    );
    // ${include(tamias:…)} whole-string form injects the parsed doc.
    let campaign = load_with_store(
        "name: test\nnodes:\n  - id: n\n    driver: shell\n    on: [start]\n    params:\n      env: ${include(tamias:handl-gates//env.yaml)}",
        tmp.path(),
        &store,
    )
    .unwrap();
    assert_eq!(
        campaign.nodes[0].params["env"],
        serde_json::json!({"REGION": "us-east-1"})
    );
    // `..` may not escape the bundle root.
    let err = load_with_store(
        "name: test\nnodes:\n  - id: n\n    on: [start]\n    params:\n      include: tamias:handl-gates//../ghost@9.9/env.yaml",
        tmp.path(),
        &store,
    )
    .unwrap_err();
    assert!(err.reason().contains("escapes the bundle root"), "{err}");
}

// --- tamias: fetch-on-miss ---------------------------------------------------
//
// A store miss with a ref-shaped spec (`git:`/`cf:`/`http(s)`/`<registry>:`/
// path) installs through `tamias-install`'s pipeline and commits the store
// entry; store-name shapes stay store-only. `load_str_with_tamias_roots`
// injects both roots so fetches never touch the real `~/.local|cache`.

/// A loadable campaign pulling `frag.yaml` out of `spec`.
fn tamias_campaign(spec: &str) -> String {
    format!(
        "name: test\nnodes:\n  - id: n\n    on: [start]\n    params:\n      include: tamias:{spec}//frag.yaml"
    )
}

fn load_with_roots(
    text: &str,
    dir: &Path,
    roots: &peregrin_loader_yaml::TamiasRoots,
) -> Result<peregrin_core::Campaign, peregrin_loader::LoadError> {
    YamlLoader.load_str_with_tamias_roots(text, dir, roots)
}

#[test]
fn tamias_include_fetches_an_fs_ref_on_store_miss() {
    let tmp = tempfile::tempdir().unwrap();
    // An installable fs ref — a package dir with a `tamias.yaml`
    // manifest and the fragment the include pulls.
    let pkg = tmp.path().join("pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(
        pkg.join("tamias.yaml"),
        "name: fetched-bundle\nversion: 1.0.0\n",
    )
    .unwrap();
    fs::write(pkg.join("frag.yaml"), "timeout: 77\n").unwrap();
    let roots =
        peregrin_loader_yaml::TamiasRoots::new(tmp.path().join("store"), tmp.path().join("cache"));
    let spec = format!("{}", pkg.display());
    let campaign = load_with_roots(&tamias_campaign(&spec), tmp.path(), &roots).unwrap();
    assert_eq!(campaign.nodes[0].params["timeout"], 77);
    // The fetch committed a provenance-carrying store entry.
    let entries = tamias_store_local::Store::new(tmp.path().join("store"))
        .entries()
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].record.name, "fetched-bundle");
    assert_eq!(entries[0].record.source_ref, spec);
    // The source dir is gone — the *name* spec now resolves store-side.
    fs::remove_dir_all(&pkg).unwrap();
    let campaign = load_with_roots(&tamias_campaign("fetched-bundle"), tmp.path(), &roots).unwrap();
    assert_eq!(campaign.nodes[0].params["timeout"], 77);
}

#[test]
fn tamias_include_fetch_failure_is_loud() {
    let tmp = tempfile::tempdir().unwrap();
    let roots =
        peregrin_loader_yaml::TamiasRoots::new(tmp.path().join("store"), tmp.path().join("cache"));
    // Ref-shaped but unfetchable — a missing fs path fails the install.
    let err = load_with_roots(
        &tamias_campaign("/nonexistent-pkg-peregrin-test"),
        tmp.path(),
        &roots,
    )
    .unwrap_err();
    assert!(err.reason().contains("fetch failed"), "{err}");
    assert!(
        err.reason().contains("/nonexistent-pkg-peregrin-test"),
        "{err}"
    );
    // An unknown registry label reaches registry resolution and fails
    // there — never a silent fallback.
    let err = load_with_roots(
        &tamias_campaign("peregrin-test-no-such-reg:pkg@1.0.0"),
        tmp.path(),
        &roots,
    )
    .unwrap_err();
    assert!(err.reason().contains("unknown registry"), "{err}");
    // Store-name shapes never reach the fetch path — the miss error
    // keeps the store-only contract.
    for spec in ["ghost", "ghost@9.9.9", "@scope/ghost"] {
        let err = load_with_roots(&tamias_campaign(spec), tmp.path(), &roots).unwrap_err();
        assert!(err.reason().contains("store-only"), "{spec}: {err}");
        assert!(err.reason().contains("tamias install"), "{spec}: {err}");
    }
}

/// Env marker for the respawned child — edition-2024 forbids
/// `env::set_var`, so env-dependent tests re-exec the test binary with
/// the vars set (the pattern `peregrin-driver-shell` uses).
const REG_CHILD: &str = "PERE_TAMIAS_REG_CHILD";

#[test]
fn tamias_include_named_registry_fetch() {
    if std::env::var_os(REG_CHILD).is_some() {
        named_registry_child();
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // A free loopback port for the fixture registry's dev http surface.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let home = root.join("home");
    let cache = root.join("cache");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cache).unwrap();
    // `TAMIAS_CONFIG` — the explicit-file leg of tamias's config chain.
    let config = root.join("tamias.toml");
    fs::write(
        &config,
        format!("[registries.fixture]\nkind = \"http\"\nurl = \"http://127.0.0.1:{port}\"\n"),
    )
    .unwrap();
    let exe = std::env::current_exe().unwrap();
    let test = std::thread::current().name().unwrap().to_string();
    let out = std::process::Command::new(exe)
        .args([test.as_str(), "--exact", "--nocapture"])
        .env(REG_CHILD, "1")
        .env("TAMIAS_CONFIG", &config)
        .env("TAMIAS_HOME", &home)
        .env("TAMIAS_CACHE", &cache)
        .env("PERE_TAMIAS_REG_ROOT", root)
        .env("PERE_TAMIAS_REG_PORT", port.to_string())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inner run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The respawned half — builds + publishes + serves a fixture registry,
/// then loads `tamias:fixture:reg-pkg@1.4.0//frag.yaml` through the
/// env-derived roots (`TAMIAS_HOME`/`TAMIAS_CACHE`/`TAMIAS_CONFIG`).
fn named_registry_child() {
    use std::path::PathBuf;

    use tamias_registry::Registry as _;

    let root = PathBuf::from(std::env::var_os("PERE_TAMIAS_REG_ROOT").unwrap());
    let port = std::env::var("PERE_TAMIAS_REG_PORT").unwrap();
    // Publish `reg-pkg@1.4.0` into a local registry dir, then serve it
    // over the dev http surface (`GET /<key>`, `PUT /<key>`).
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("tamias.yaml"), "name: reg-pkg\nversion: 1.4.0\n").unwrap();
    fs::write(src.join("frag.yaml"), "timeout: 88\n").unwrap();
    let reg = root.join("registry-store");
    let manifest = tamias_core::Manifest::load(&src).unwrap();
    tamias_registry_local::LocalRegistry::new(&reg)
        .publish(&src, &manifest, None)
        .unwrap();
    let addr = format!("127.0.0.1:{port}");
    let served = reg.clone();
    let serve_addr = addr.clone();
    std::thread::spawn(move || {
        let _ = tamias_registry_local::serve_http(&serve_addr, &served);
    });
    for _ in 0..100 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // The load: env-derived store (`$TAMIAS_HOME/store`) + cache
    // (`$TAMIAS_CACHE`) + config (`$TAMIAS_CONFIG`).
    let tmp = tempfile::tempdir().unwrap();
    let campaign = YamlLoader
        .load_str(&tamias_campaign("fixture:reg-pkg@1.4.0"), tmp.path())
        .unwrap();
    assert_eq!(campaign.nodes[0].params["timeout"], 88);
    // The store commit recorded the verbatim registry ref.
    let store = PathBuf::from(std::env::var_os("TAMIAS_HOME").unwrap()).join("store");
    let entries = tamias_store_local::Store::new(&store).entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].record.name, "reg-pkg");
    assert_eq!(entries[0].record.source_ref, "fixture:reg-pkg@1.4.0");
}
