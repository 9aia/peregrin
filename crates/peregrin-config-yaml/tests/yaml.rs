//! YamlSource: reads one file as YAML — same campaign-shaped node
//! config through the `serde_norway` transcode: `Value` →
//! `serde_json::Value` → `Campaign::from_dict`. Full shape, `~`
//! expansion, missing-file error, parse error, `from_dict` shape
//! errors, non-mapping documents.
#![allow(clippy::unwrap_used)]

use peregrin_config::ConfigSource;
use peregrin_config_yaml::YamlSource;
use std::path::Path;

fn load(p: &Path) -> Result<peregrin_core::Campaign, String> {
    YamlSource {
        path: p.to_path_buf(),
    }
    .load()
}

/// Full shape: name/config/events/nodes (incl. an ingress-driver
/// node)/functions-as-mapping/include+import file lists with `~`/
/// stop_on/strict/state/annotations/payload_schemas/capabilities —
/// the ADR-0080 node config in YAML.
const FULL: &str = r#"
name: ci-gate
strict: true
stop_on: [run.failed]
include:
  - ~/bundles/extra.yaml
  - ci-gate
import:
  - file: ~/lib/helpers.py
    as: helpers
config:
  context: ci/peregrin
events:
  - type: push.verified
    fields:
      sha: string
nodes:
  - id: webhook
    driver: github-webhook
    emits: [push.verified]
    params:
      repos: ["9aia/peregrin"]
      secret_env: GATE_SECRET
  - id: run
    driver: git
    on: [push.verified]
    emits: [run.done]
    params:
      inner: shell
functions:
  notify:
    driver: http
    params:
      url: https://example.com/hook
state:
  counter: 0
annotations:
  owner: factory
payload_schemas:
  push.verified:
    type: object
capabilities:
  exec: true
"#;

#[test]
fn yaml_full_shape_and_tilde() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path().join("peregrin.yaml");
    std::fs::write(&p, FULL).unwrap();
    let home = std::env::var("HOME").unwrap();
    let c = load(&p).unwrap();

    assert_eq!(c.name, "ci-gate");
    assert!(c.strict);
    assert_eq!(c.stop_on, ["run.failed"]);
    assert_eq!(c.config["context"], "ci/peregrin");

    assert_eq!(c.events[0].kind, "push.verified");
    assert_eq!(c.events[0].fields["sha"], "string");

    let webhook = &c.nodes[0];
    assert_eq!(webhook.id, "webhook");
    assert_eq!(webhook.driver, "github-webhook");
    assert_eq!(webhook.emits, ["push.verified"]);
    assert_eq!(webhook.params["repos"][0], "9aia/peregrin");
    assert_eq!(webhook.params["secret_env"], "GATE_SECRET");

    let run = &c.nodes[1];
    assert_eq!(run.driver, "git");
    assert_eq!(run.on, ["push.verified"]);
    assert_eq!(run.emits, ["run.done"]);

    assert_eq!(c.functions[0].name, "notify");
    assert_eq!(c.functions[0].driver, "http");

    // `~` expanded in the file lists; bundle names pass through.
    assert_eq!(c.imports[0], format!("{home}/bundles/extra.yaml"));
    assert_eq!(c.imports[1], "ci-gate");
    assert_eq!(c.python_imports[0].file, format!("{home}/lib/helpers.py"));
    assert_eq!(c.python_imports[0].alias, "helpers");

    assert_eq!(c.state["counter"], 0);
    assert_eq!(c.annotations["owner"], "factory");
    assert_eq!(c.payload_schemas["push.verified"]["type"], "object");
    assert_eq!(c.capabilities["exec"], true);
}

#[test]
fn missing_and_malformed_err() {
    let dir = tempfile::TempDir::new().unwrap();
    let e = load(&dir.path().join("nope.yaml")).unwrap_err();
    assert!(e.contains("read"), "{e}");

    // Malformed YAML -> parse error.
    let p = dir.path().join("peregrin.yaml");
    std::fs::write(&p, "nodes: [unclosed\n").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("parse"), "{e}");
}

#[test]
fn shape_errors_are_parse_errors() {
    let dir = tempfile::TempDir::new().unwrap();

    // Non-mapping document -> "must be a mapping".
    let p = dir.path().join("list.yaml");
    std::fs::write(&p, "- 1\n- 2\n").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("must be a mapping"), "{e}");
    assert!(e.contains("parse"), "{e}");

    // `nodes:` present but not an array -> `from_dict` hard error.
    let p = dir.path().join("badnodes.yaml");
    std::fs::write(&p, "nodes:\n  a: 1\n").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("'nodes' must be an array"), "{e}");
}
