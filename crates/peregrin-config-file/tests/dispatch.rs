//! FileSource dispatch: the extension picks the syntax adapter —
//! `.yaml`/`.yml` → YamlSource, everything else → TomlSource
//! (back-compat for explicit/extensionless paths) — and adapter
//! errors surface through the dispatcher.
#![allow(clippy::unwrap_used)]

use peregrin_config::ConfigSource;
use peregrin_config_file::FileSource;
use std::path::Path;

fn load(p: &Path) -> Result<peregrin_core::Campaign, String> {
    FileSource {
        path: Some(p.to_string_lossy().into_owned()),
    }
    .load()
}

#[test]
fn explicit_paths_dispatch_by_extension() {
    let dir = tempfile::TempDir::new().unwrap();

    // .toml -> TOML semantics.
    let p = dir.path().join("peregrin.toml");
    std::fs::write(&p, "name = \"via-toml\"\n").unwrap();
    assert_eq!(load(&p).unwrap().name, "via-toml");

    // .yaml -> YAML semantics.
    let p = dir.path().join("peregrin.yaml");
    std::fs::write(&p, "name: via-yaml\n").unwrap();
    assert_eq!(load(&p).unwrap().name, "via-yaml");

    // .yml -> YAML semantics.
    let p = dir.path().join("peregrin.yml");
    std::fs::write(&p, "name: via-yml\n").unwrap();
    assert_eq!(load(&p).unwrap().name, "via-yml");

    // Extensionless explicit path -> TOML (back-compat).
    let p = dir.path().join("peregrin");
    std::fs::write(&p, "name = \"via-bare\"\n").unwrap();
    assert_eq!(load(&p).unwrap().name, "via-bare");

    // A `.toml` path keeps TOML semantics even when the bytes would
    // also parse as YAML — the extension picks the adapter.
    let p = dir.path().join("ambig.toml");
    std::fs::write(&p, "name = \"toml-wins\"\n").unwrap();
    assert_eq!(load(&p).unwrap().name, "toml-wins");
}

#[test]
fn missing_and_malformed_err() {
    let dir = tempfile::TempDir::new().unwrap();

    // Explicit path that doesn't exist -> the miss error.
    let e = load(&dir.path().join("nope.toml")).unwrap_err();
    assert!(e.contains("no peregrin.{yaml,yml,toml}"), "{e}");

    // Malformed file -> the adapter's parse error surfaces.
    let p = dir.path().join("peregrin.toml");
    std::fs::write(&p, "[[[").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("parse"), "{e}");

    let p = dir.path().join("peregrin.yaml");
    std::fs::write(&p, "nodes: [unclosed\n").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("parse"), "{e}");

    // exists() but unreadable-as-file (a directory) -> read arm.
    let p = dir.path().join("dir-as-cfg.yaml");
    std::fs::create_dir(&p).unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("read"), "{e}");
}

/// TOML/YAML parity: the same logical config parses to the same
/// `Campaign` through either adapter.
#[test]
fn toml_yaml_parity() {
    let dir = tempfile::TempDir::new().unwrap();
    let toml_path = dir.path().join("peregrin.toml");
    let yaml_path = dir.path().join("peregrin.yaml");
    std::fs::write(
        &toml_path,
        r#"name = "gate"
strict = true
stop_on = ["run.failed"]
include = ["bundles/x.yaml"]

[config]
context = "ci/peregrin"

[[nodes]]
id = "webhook"
driver = "github-webhook"
emits = ["push.verified"]
params = { repos = ["9aia/peregrin"], secret_env = "GATE_SECRET" }
"#,
    )
    .unwrap();
    std::fs::write(
        &yaml_path,
        r#"name: gate
strict: true
stop_on: [run.failed]
include: [bundles/x.yaml]
config:
  context: ci/peregrin
nodes:
  - id: webhook
    driver: github-webhook
    emits: [push.verified]
    params:
      repos: ["9aia/peregrin"]
      secret_env: GATE_SECRET
"#,
    )
    .unwrap();
    assert_eq!(load(&toml_path).unwrap(), load(&yaml_path).unwrap());
}
