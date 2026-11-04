//! TomlSource: reads one file as TOML — the full campaign-shaped
//! node config (ADR-0080), `~` expansion on the file lists,
//! missing-file error, parse error, `from_dict` shape errors.
#![allow(clippy::unwrap_used)]

use peregrin_config::ConfigSource;
use peregrin_config_toml::TomlSource;
use std::path::Path;

fn load(p: &Path) -> Result<peregrin_core::Campaign, String> {
    TomlSource {
        path: p.to_path_buf(),
    }
    .load()
}

/// Full shape: name/config/events/nodes (incl. an ingress-driver
/// node)/functions-as-mapping/include+import file lists/
/// stop_on/strict/state/annotations/payload_schemas/capabilities —
/// the ADR-0080 node config in TOML.
const FULL: &str = r#"
name = "ci-gate"
strict = true
stop_on = ["run.failed"]
include = ["bundles/extra.yaml", "ci-gate"]
import = [{ file = "lib/helpers.py", as = "helpers" }]

[config]
context = "ci/peregrin"

[[events]]
type = "push.verified"
fields = { sha = "string" }

[[nodes]]
id = "webhook"
driver = "github-webhook"
emits = ["push.verified"]
params = { repos = ["9aia/peregrin"], secret_env = "GATE_SECRET" }

[[nodes]]
id = "run"
driver = "git"
on = ["push.verified"]
emits = ["run.done"]
params = { inner = "shell" }

[functions.notify]
driver = "http"
params = { url = "https://example.com/hook" }

[state]
counter = 0

[annotations]
owner = "factory"

[payload_schemas]
"push.verified" = { type = "object" }

[capabilities]
exec = true
"#;

#[test]
fn loads_toml_full_shape() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path().join("peregrin.toml");
    std::fs::write(&p, FULL).unwrap();
    let c = load(&p).unwrap();

    assert_eq!(c.name, "ci-gate");
    assert!(c.strict);
    assert_eq!(c.stop_on, ["run.failed"]);
    assert_eq!(c.config["context"], "ci/peregrin");

    assert_eq!(c.events[0].kind, "push.verified");
    assert_eq!(c.events[0].fields["sha"], "string");

    // The ingress-driver node — params stay an untyped dict.
    let webhook = &c.nodes[0];
    assert_eq!(webhook.id, "webhook");
    assert_eq!(webhook.driver, "github-webhook");
    assert_eq!(webhook.emits, ["push.verified"]);
    assert_eq!(webhook.params["repos"][0], "9aia/peregrin");
    assert_eq!(webhook.params["secret_env"], "GATE_SECRET");

    let run = &c.nodes[1];
    assert_eq!(run.driver, "git");
    assert_eq!(run.on, ["push.verified"]);

    // `functions:` in mapping form normalizes to records.
    assert_eq!(c.functions[0].name, "notify");
    assert_eq!(c.functions[0].driver, "http");
    assert_eq!(c.functions[0].params["url"], "https://example.com/hook");

    assert_eq!(c.imports, ["bundles/extra.yaml", "ci-gate"]);
    assert_eq!(c.python_imports[0].file, "lib/helpers.py");
    assert_eq!(c.python_imports[0].alias, "helpers");

    assert_eq!(c.state["counter"], 0);
    assert_eq!(c.annotations["owner"], "factory");
    assert_eq!(c.payload_schemas["push.verified"]["type"], "object");
    assert_eq!(c.capabilities["exec"], true);
}

#[test]
fn tilde_expands_in_file_lists() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path().join("peregrin.toml");
    std::fs::write(
        &p,
        "include = [\"~/bundles/x.yaml\", \"~root/y.yaml\"]\nimport = [{ file = \"~/m.py\" }]\n",
    )
    .unwrap();
    let home = std::env::var("HOME").unwrap();
    let c = load(&p).unwrap();
    assert_eq!(c.imports[0], format!("{home}/bundles/x.yaml"));
    // `~user` is not expanded.
    assert_eq!(c.imports[1], "~root/y.yaml");
    assert_eq!(c.python_imports[0].file, format!("{home}/m.py"));
}

#[test]
fn missing_and_malformed_err() {
    let dir = tempfile::TempDir::new().unwrap();
    let e = load(&dir.path().join("nope.toml")).unwrap_err();
    assert!(e.contains("read"), "{e}");

    let p = dir.path().join("peregrin.toml");
    std::fs::write(&p, "[[[").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("parse"), "{e}");
}

#[test]
fn shape_errors_are_parse_errors() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path().join("peregrin.toml");
    // `nodes` present but not an array — the `from_dict` hard error.
    std::fs::write(&p, "nodes = { a = 1 }\n").unwrap();
    let e = load(&p).unwrap_err();
    assert!(e.contains("parse"), "{e}");
    assert!(e.contains("'nodes' must be an array"), "{e}");
}
