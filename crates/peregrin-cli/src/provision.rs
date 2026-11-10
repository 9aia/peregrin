//! `peregrin provision …` — the tamias **content-type provider** entrypoint.
//!
//! tamias owns distribution/provisioning/deploy as *generic* verbs; it
//! shells `peregrin provision {install,deploy,uninstall} --dir <dir>
//! --name <name> [--config <path>]` (via `tamias-driver-peregrin`) for
//! the peregrin-specific bits — the node config — and never learns
//! peregrin's schema. Service units are *not* written here: the
//! bundle's `tamias.yaml` `provider.services[]` block is rendered into
//! per-user units by `tamias deploy` itself (`<provider>-<service>`).
//!
//! - `install`   — write `<name>.yaml` (a node config `include:`ing the
//!   bundle's `main.yaml`) from an installed bundle dir. The tamias
//!   driver's `install` verb is a no-op (it only sees the evictable
//!   fetch-cache payload); this stays for `peregrin provision install`.
//! - `deploy`    — (re)generate `<name>.yaml` as the full operator node
//!   config: the bundle `include:` plus a `github-webhook` ingress node
//!   and `config.public_url`/`capabilities.secrets` derived from the
//!   manifest's `provider.ingress[]` declaration. Forge-hook
//!   *registration* is not performed — the ingress block is data for a
//!   later driver pass.
//! - `uninstall` — remove the node config + any legacy `<name>.service`
//!   unit a pre-provider `deploy` may have written (provider units are
//!   removed by `tamias uninstall` itself).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::CliError;

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(|d| PathBuf::from(d).join("peregrin"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/peregrin")))
        .unwrap_or_else(|| PathBuf::from(".config/peregrin"))
}

fn config_path(name: &str, config: Option<&Path>) -> PathBuf {
    config.map_or_else(
        || config_dir().join(format!("{name}.yaml")),
        Path::to_path_buf,
    )
}

/// The bundle's campaign entrypoint — `main.yaml`/`main.yml` inside the
/// installed payload dir.
fn bundle_main(dir: &Path) -> Result<PathBuf, CliError> {
    ["main.yaml", "main.yml"]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| CliError(format!("bundle dir {} has no main.yaml", dir.display())))
}

fn write_config(path: &Path, doc: &Value) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CliError(e.to_string()))?;
    }
    let text = serde_yaml_ng::to_string(doc).map_err(|e| CliError(e.to_string()))?;
    std::fs::write(path, text).map_err(|e| CliError(e.to_string()))
}

/// `PEREGRIN_WH_<REPO>` — the webhook-secret env name convention:
/// `owner/repo` uppercased, every non-alphanumeric byte a `_`
/// (`9aia/peregrin` → `PEREGRIN_WH_9AIA_PEREGRIN`). Names only — the
/// value lives in `~/.config/peregrin/env` (or the keyring).
fn webhook_secret_env(repo: &str) -> String {
    let slug: String = repo
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("PEREGRIN_WH_{slug}")
}

/// The operator node config `deploy` generates — a campaign document
/// that `include:`s the bundle's `main.yaml` (policy) and adds the
/// ingress half the bundle deliberately never ships (ADR-0080): one
/// `github-webhook` node whose `params.repos` come from the manifest's
/// `provider.ingress[].hooks` (`forge: github` entries only — other
/// forges are declared data until their drivers exist).
fn deploy_config_doc(name: &str, main: &Path, dir: &Path) -> Value {
    let mut doc = Map::new();
    doc.insert("name".into(), json!(name));
    doc.insert(
        "annotations".into(),
        json!({"generated_by": "tamias deploy (peregrin provider)"}),
    );
    doc.insert("include".into(), json!([main.display().to_string()]));

    let mut config = Map::new();
    config.insert("name".into(), json!(name));

    // provider.ingress[] → the webhook node's repos + the public URL
    // the tunnel fronts. Absent/unparseable manifest → the bare
    // include+config doc `install` writes (a plain bundle has no
    // ingress to bind).
    let manifest = tamias_core::Manifest::load(dir).ok();
    let provider = manifest.as_ref().and_then(|m| m.provider.as_ref());
    if let Some(provider) = provider {
        let ingress = provider.ingress.first();
        if let Some(url) = ingress
            .map(|i| i.public_url.as_str())
            .filter(|u| !u.is_empty())
        {
            config.insert("public_url".into(), json!(url));
        }
        // repos: every github hook across all ingress entries, deduped
        // in declaration order.
        let mut repos: Vec<String> = Vec::new();
        for hook in provider
            .ingress
            .iter()
            .flat_map(|i| i.hooks.iter())
            .filter(|h| h.forge.eq_ignore_ascii_case("github"))
        {
            if !repos.contains(&hook.repo) {
                repos.push(hook.repo.clone());
            }
        }
        if !repos.is_empty() {
            let mut secrets: Vec<Value> = Vec::new();
            // The bundle's forge token — `config.token_env` defaults to
            // GITHUB_TOKEN; declare it so the keyring leg may serve it.
            secrets.push(json!("GITHUB_TOKEN"));
            for repo in &repos {
                let env_name = webhook_secret_env(repo);
                if !secrets
                    .iter()
                    .any(|s| s.as_str() == Some(env_name.as_str()))
                {
                    secrets.push(json!(env_name));
                }
            }
            doc.insert(
                "capabilities".into(),
                json!({"secrets": secrets.iter().map(Value::as_str).collect::<Vec<_>>()}),
            );

            let mut params = Map::new();
            if let Some(listen) = ingress.and_then(|i| i.listen.as_deref()) {
                params.insert("listen".into(), json!(listen));
            }
            if let Some(path) = ingress.and_then(|i| i.path.as_deref()) {
                params.insert("path".into(), json!(path));
            }
            params.insert(
                "repos".into(),
                Value::Array(
                    repos
                        .iter()
                        .map(|repo| {
                            json!({
                                "name": repo,
                                "secret_env": webhook_secret_env(repo),
                            })
                        })
                        .collect(),
                ),
            );
            doc.insert(
                "nodes".into(),
                json!([{
                    "id": format!("{name}.ingress"),
                    "driver": "github-webhook",
                    "on": ["start"],
                    "emits": [format!("{name}.ingress.listening"), "push"],
                    "params": Value::Object(params),
                }]),
            );
        }
    }
    doc.insert("config".into(), Value::Object(config));
    Value::Object(doc)
}

/// `install` — write a node config that `include:`s the bundle's main.yaml.
///
/// # Errors
/// Fails when the bundle dir has no `main.yaml` or the write fails.
pub fn install(dir: &Path, name: &str, config: Option<&Path>) -> Result<String, CliError> {
    let main = bundle_main(dir)?;
    let path = config_path(name, config);
    let mut doc = Map::new();
    doc.insert("name".into(), json!(name));
    doc.insert(
        "annotations".into(),
        json!({"generated_by": "tamias install (peregrin provider)"}),
    );
    doc.insert("include".into(), json!([main.display().to_string()]));
    doc.insert("config".into(), json!({"name": name}));
    write_config(&path, &Value::Object(doc))?;
    Ok(format!("wrote node config {}", path.display()))
}

/// `deploy` — (re)generate the operator node config from the bundle's
/// `provider:` block. No unit is written: `tamias deploy` renders
/// `provider.services[]` into `<provider>-<service>` units itself.
/// Regenerating overwrites operator hand-edits — the manifest is the
/// source of truth.
///
/// # Errors
/// Fails when the bundle dir has no `main.yaml` or the write fails.
pub fn deploy(dir: &Path, name: &str, config: Option<&Path>) -> Result<String, CliError> {
    let main = bundle_main(dir)?;
    let path = config_path(name, config);
    let doc = deploy_config_doc(name, &main, dir);
    write_config(&path, &doc)?;
    Ok(format!(
        "wrote node config {} (units render via `tamias deploy {name}`)",
        path.display()
    ))
}

/// `uninstall` — remove the node config + any legacy `<name>.service`
/// unit a pre-provider `deploy` wrote. Provider-declared units
/// (`<provider>-<service>`) are `tamias uninstall`'s job.
///
/// # Errors
/// Best-effort — always returns Ok unless the name is empty.
pub fn uninstall(name: &str, config: Option<&Path>) -> Result<String, CliError> {
    let path = config_path(name, config);
    if path.is_file() {
        let _ = std::fs::remove_file(&path);
    }
    let _ = hypha_systemd::remove(&hypha_systemd::default_unit_dir(), name);
    Ok(format!("removed config + legacy unit for {name}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// A bundle dir with `main.yaml` + a `tamias.yaml` carrying a
    /// `provider:` block — the ci-gate pilot shape.
    fn bundle_dir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("ci-gate@0.1.0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.yaml"), "name: ci-gate\nnodes: []\n").unwrap();
        std::fs::write(
            dir.join("tamias.yaml"),
            r#"name: ci-gate
version: "0.1.0"
type: peregrin-bundle
provider:
  name: ci-gate
  services:
    - name: campaign
      exec: "peregrin campaign serve ~/.config/peregrin/ci-gate.yaml"
  ingress:
    - public_url: https://ci.example.com
      listen: 127.0.0.1:39271
      path: /hook
      hooks:
        - forge: github
          repo: 9aia/peregrin
          events: [push]
        - forge: github
          repo: 9aia/other
          events: [push]
        - forge: gitlab
          repo: ignored/group
          events: [push]
  verbs: [run, logs]
"#,
        )
        .unwrap();
        (tmp, dir)
    }

    #[test]
    fn install_writes_a_config() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bundle");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.yaml"), "name: x\n").unwrap();
        let cfg = tmp.path().join("x.yaml");
        let line = install(&dir, "x", Some(&cfg)).unwrap();
        assert!(line.contains("wrote node config"));
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("include:"));
    }

    #[test]
    fn deploy_generates_the_operator_node_config() {
        let (tmp, dir) = bundle_dir();
        let cfg = tmp.path().join("ci-gate.yaml");
        let line = deploy(&dir, "ci-gate", Some(&cfg)).unwrap();
        assert!(line.contains("wrote node config"));
        let text = std::fs::read_to_string(&cfg).unwrap();
        // include the bundle's main.yaml (absolute store path)
        assert!(text.contains(&dir.join("main.yaml").display().to_string()));
        // public_url + ingress node with github repos only
        assert!(text.contains("https://ci.example.com"));
        assert!(text.contains("github-webhook"));
        assert!(text.contains("9aia/peregrin"));
        assert!(text.contains("9aia/other"));
        assert!(!text.contains("ignored/group"));
        // secret_env names follow the PEREGRIN_WH_ convention
        assert!(text.contains("PEREGRIN_WH_9AIA_PEREGRIN"));
        assert!(text.contains("PEREGRIN_WH_9AIA_OTHER"));
        // listen/path ride the node params
        assert!(text.contains("127.0.0.1:39271"));
        assert!(text.contains("/hook"));
        // the generated doc parses back as yaml
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
        assert_eq!(parsed["name"].as_str(), Some("ci-gate"));
        assert_eq!(
            parsed["config"]["public_url"].as_str(),
            Some("https://ci.example.com")
        );
    }

    #[test]
    fn deploy_without_provider_falls_back_to_bare_config() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("plain");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.yaml"), "name: x\n").unwrap();
        // no tamias.yaml at all → manifest load fails → bare doc
        let cfg = tmp.path().join("plain.yaml");
        deploy(&dir, "plain", Some(&cfg)).unwrap();
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("include:"));
        assert!(!text.contains("github-webhook"));
        assert!(!text.contains("capabilities"));
    }

    #[test]
    fn webhook_secret_env_slug() {
        assert_eq!(
            webhook_secret_env("9aia/peregrin"),
            "PEREGRIN_WH_9AIA_PEREGRIN"
        );
        assert_eq!(webhook_secret_env("o/r-n.x"), "PEREGRIN_WH_O_R_N_X");
    }

    /// The shipped ci-gate bundle's manifest must keep validating —
    /// guards the `provider:` block against schema drift.
    #[test]
    fn shipped_ci_gate_manifest_validates() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/ci-gate");
        let manifest = tamias_core::Manifest::load(&dir).unwrap();
        assert_eq!(manifest.kind.as_deref(), Some("peregrin-bundle"));
        let provider = manifest.provider.as_ref().unwrap();
        assert_eq!(provider.name.as_deref(), Some("ci-gate"));
        assert_eq!(provider.services.len(), 1);
        assert_eq!(provider.services[0].name, "campaign");
        assert!(provider.services[0].exec.contains("campaign serve"));
        assert_eq!(provider.ingress.len(), 1);
        assert_eq!(provider.ingress[0].hooks[0].repo, "9aia/peregrin");
    }
}
