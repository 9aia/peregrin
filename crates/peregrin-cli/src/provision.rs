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

// --- provider ops verbs (T8-5) -------------------------------------------------
//
// `tamias driver exec <pkg> hooks|health|status|logs` reaches these via
// `tamias-driver-peregrin`. Everything here is manifest-driven: the
// store payload's `tamias.yaml` `provider:` block is the source of
// truth — hook targets come from `ingress[].hooks[]`, units from
// `services[]`. Forge API calls reuse `ops`' GitHub helpers (upsert by
// exact hook-URL match); secrets resolve by NAME through the same
// env → keyring → env-file chain — values are never printed.

/// One concrete forge-hook target resolved from the manifest —
/// `<public_url><path>` per `ingress[].hooks[]` entry.
#[derive(Clone, Debug)]
struct HookTarget {
    /// `owner/repo`.
    repo: String,
    /// The hook URL GitHub delivers to (`public_url` + `path`).
    hook_url: String,
    /// Subscribed event types (`push`, …; empty → `["push"]`).
    events: Vec<String>,
    /// The webhook-secret NAME (`PEREGRIN_WH_<OWNER>_<REPO>`).
    secret_env: String,
    /// The forge REST base (`--api-base`, default github.com).
    api_base: String,
}

/// The manifest's github forge hooks flattened to registration
/// targets — non-github forges stay declared data (their drivers
/// don't exist yet). Deduped by `(repo, hook_url)`, declaration order.
fn hook_targets(
    dir: &Path,
    api_base: Option<&str>,
    verb: &str,
) -> Result<Vec<HookTarget>, CliError> {
    let manifest = tamias_core::Manifest::load(dir)
        .map_err(|e| CliError(format!("{}: no readable tamias.yaml — {e}", dir.display())))?;
    let provider = manifest.provider.as_ref().ok_or_else(|| {
        CliError(format!(
            "{}: no provider: block — nothing to manage",
            dir.display()
        ))
    })?;
    let base = api_base.unwrap_or(crate::ops::DEFAULT_API_BASE);
    let mut targets: Vec<HookTarget> = Vec::new();
    for ingress in &provider.ingress {
        let url = ingress.public_url.trim_end_matches('/');
        if url.is_empty() {
            continue;
        }
        let path = ingress
            .path
            .as_deref()
            .unwrap_or(crate::ops::DEFAULT_HOOK_PATH);
        let hook_url = format!("{url}{path}");
        for hook in &ingress.hooks {
            if !hook.forge.eq_ignore_ascii_case("github") {
                continue;
            }
            let events = if hook.events.is_empty() {
                vec!["push".to_string()]
            } else {
                hook.events.clone()
            };
            let target = HookTarget {
                repo: hook.repo.clone(),
                hook_url: hook_url.clone(),
                events,
                secret_env: webhook_secret_env(&hook.repo),
                api_base: base.to_string(),
            };
            if !targets
                .iter()
                .any(|t| t.repo == target.repo && t.hook_url == target.hook_url)
            {
                targets.push(target);
            }
        }
    }
    if targets.is_empty() {
        return Err(CliError(format!(
            "{}: provider.ingress declares no github hooks — nothing to {verb}",
            dir.display()
        )));
    }
    Ok(targets)
}

/// `hooks register|unregister` — upsert/remove the forge webhooks the
/// manifest's `provider.ingress[].hooks[]` declare. `register` is
/// idempotent (the ops upsert matches on the exact hook URL); both
/// directions need the forge token (secret NAME `--token-env`, default
/// `GITHUB_TOKEN`) and — for register — each repo's webhook secret by
/// NAME. `--dry-run` prints the plan and touches neither the forge API
/// nor the secret chain.
///
/// # Errors
/// Fails on an unreadable manifest, missing secrets, or forge API errors.
/// Returns the report lines joined by newlines (driver-verb output).
pub fn hooks(
    dir: &Path,
    env: &crate::ops::OpsEnv,
    register: bool,
    dry_run: bool,
    api_base: Option<&str>,
    token_env: Option<&str>,
) -> Result<String, CliError> {
    let verb = if register { "register" } else { "unregister" };
    let targets = hook_targets(dir, api_base, verb)?;
    if dry_run {
        return Ok(targets
            .iter()
            .map(|t| {
                format!(
                    "{}: would {} {} events=[{}] secret={} (name — value never shown)",
                    t.repo,
                    verb,
                    t.hook_url,
                    t.events.join(","),
                    t.secret_env
                )
            })
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let token_name = token_env.unwrap_or(crate::ops::DEFAULT_TOKEN_ENV);
    let token = crate::ops::secret(env, token_name)
        .ok_or_else(|| crate::ops::missing_secret(env, token_name))?;
    let mut lines = Vec::with_capacity(targets.len());
    for target in &targets {
        let msg = if register {
            let hook_secret = crate::ops::secret(env, &target.secret_env)
                .ok_or_else(|| crate::ops::missing_secret(env, &target.secret_env))?;
            let events: Vec<&str> = target.events.iter().map(String::as_str).collect();
            crate::ops::register_hook(
                &token,
                &target.repo,
                &target.hook_url,
                &hook_secret,
                &target.api_base,
                &events,
            )?
        } else {
            crate::ops::remove_hook(&token, &target.repo, &target.hook_url, &target.api_base)?
        };
        lines.push(msg);
    }
    Ok(lines.join("\n"))
}

/// The provider's unit base names — `<provider-or-package>-<service>`
/// (the same derivation `tamias deploy` renders, kept in sync with
/// `provider_unit_name` there).
fn provider_units(manifest: &tamias_core::Manifest) -> Vec<(String, String)> {
    let Some(provider) = manifest.provider.as_ref() else {
        return Vec::new();
    };
    let base = provider
        .name
        .clone()
        .unwrap_or_else(|| match &manifest.scope {
            Some(scope) => format!("{scope}-{}", manifest.name),
            None => manifest.name.clone(),
        });
    provider
        .services
        .iter()
        .map(|s| (s.name.clone(), format!("{base}-{}", s.name)))
        .collect()
}

/// `systemctl --user <args>` — bounded at 5s; `None` when systemd
/// can't answer (no user bus, timeout, missing binary).
fn systemctl_user(args: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg("--user").args(args);
    let out = tamias_core::output_with_timeout(&mut cmd, std::time::Duration::from_secs(5)).ok()?;
    unit_output(&out)
}

/// stdout trimmed, else stderr trimmed, else `None` — the
/// `is-active`/`is-enabled` state read (exit status ignored: both
/// verbs exit non-zero for `inactive`/`disabled`/`not-found`).
fn unit_output(out: &std::process::Output) -> Option<String> {
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return (!err.is_empty()).then_some(err);
    }
    Some(text)
}

/// `health` — probe the provider's ingress listeners with the
/// github-webhook driver's `ping` event (`POST <listen><path>` +
/// `X-GitHub-Event: ping` → `200 pong`), falling back to
/// `systemctl --user is-active` per provider unit when no ingress
/// declares `listen`+`path`. Prints a JSON status object.
///
/// # Errors
/// Fails when the manifest can't be read or nothing is probeable.
pub fn health(dir: &Path, name: &str) -> Result<String, CliError> {
    let manifest = tamias_core::Manifest::load(dir)
        .map_err(|e| CliError(format!("{}: no readable tamias.yaml — {e}", dir.display())))?;
    let provider = manifest.provider.as_ref().ok_or_else(|| {
        CliError(format!(
            "{}: no provider: block — nothing to probe",
            dir.display()
        ))
    })?;
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .new_agent();
    let mut probes = Vec::new();
    for ingress in &provider.ingress {
        let (Some(listen), Some(path)) = (&ingress.listen, &ingress.path) else {
            continue;
        };
        let url = format!("http://{listen}{path}");
        let result = agent
            .post(&url)
            .header("x-github-event", "ping")
            .header("User-Agent", "peregrin")
            .send_empty()
            .map_or_else(
                |e| (false, e.to_string()),
                |mut resp| {
                    let status = resp.status().as_u16();
                    let body = resp
                        .body_mut()
                        .read_to_string()
                        .unwrap_or_else(|_| String::new());
                    (status == 200, format!("{status} {}", body.trim()))
                },
            );
        probes.push(serde_json::json!({
            "kind": "hook-ping",
            "url": url,
            "ok": result.0,
            "detail": result.1,
        }));
    }
    if probes.is_empty() {
        // No HTTP probe declared — fall back to unit state.
        for (service, unit) in provider_units(&manifest) {
            let state = systemctl_user(&["is-active", &format!("{unit}.service")])
                .unwrap_or_else(|| "unknown".to_string());
            probes.push(serde_json::json!({
                "kind": "unit",
                "service": service,
                "unit": format!("{unit}.service"),
                "ok": state == "active",
                "detail": state,
            }));
        }
    }
    if probes.is_empty() {
        return Err(CliError(format!(
            "{name}: provider declares no ingress listeners or services — nothing to probe"
        )));
    }
    let ok = probes.iter().all(|p| p["ok"].as_bool() == Some(true));
    serde_json::to_string(&serde_json::json!({
        "name": name,
        "ok": ok,
        "probes": probes,
    }))
    .map_err(|e| CliError(e.to_string()))
}

/// `status` — per-unit `systemctl --user is-active`/`is-enabled` as a
/// JSON object. Units that systemd can't answer report `"unknown"`.
///
/// # Errors
/// Fails when the manifest can't be read or declares no services.
pub fn status(dir: &Path, name: &str) -> Result<String, CliError> {
    status_inner(dir, name, &systemctl_user)
}

/// [`status`] with the unit-state probe injected — tests hand a fake
/// `systemctl --user` (edition-2024 forbids `env::set_var`, so PATH
/// fakery is out).
fn status_inner(
    dir: &Path,
    name: &str,
    run: &dyn Fn(&[&str]) -> Option<String>,
) -> Result<String, CliError> {
    let manifest = tamias_core::Manifest::load(dir)
        .map_err(|e| CliError(format!("{}: no readable tamias.yaml — {e}", dir.display())))?;
    let units = provider_units(&manifest);
    if units.is_empty() {
        return Err(CliError(format!(
            "{name}: provider declares no services — nothing to report"
        )));
    }
    let rows: Vec<serde_json::Value> = units
        .iter()
        .map(|(service, unit)| {
            let full = format!("{unit}.service");
            let active = run(&["is-active", &full]).unwrap_or_else(|| "unknown".to_string());
            let enabled = run(&["is-enabled", &full]).unwrap_or_else(|| "unknown".to_string());
            serde_json::json!({
                "service": service,
                "unit": full,
                "active": active,
                "enabled": enabled,
                "ok": active == "active",
            })
        })
        .collect();
    let ok = rows.iter().all(|r| r["ok"].as_bool() == Some(true));
    serde_json::to_string(&serde_json::json!({
        "name": name,
        "ok": ok,
        "units": rows,
    }))
    .map_err(|e| CliError(e.to_string()))
}

/// `logs` — `journalctl --user -u <unit>` over every provider unit,
/// newest `lines` entries, no follow (dispatch is bounded at 120s — a
/// `-f` would just be killed). Returns the journal text.
///
/// # Errors
/// Fails when the manifest can't be read or declares no services.
pub fn logs(dir: &Path, name: &str, lines: u32) -> Result<String, CliError> {
    logs_inner(dir, name, lines, &journalctl_user)
}

/// `journalctl --user -u <unit> -n <lines> --no-pager` for one unit.
fn journalctl_user(unit: &str, lines: u32) -> Result<String, CliError> {
    let n = lines.max(1).to_string();
    let out = std::process::Command::new("journalctl")
        .args(["--user", "-u", unit, "-n", &n, "--no-pager"])
        .output()
        .map_err(|e| CliError(format!("journalctl {unit}: {e}")))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        text.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    Ok(text)
}

/// [`logs`] with the journal probe injected (same seam as
/// [`status_inner`]).
fn logs_inner(
    dir: &Path,
    name: &str,
    lines: u32,
    run: &dyn Fn(&str, u32) -> Result<String, CliError>,
) -> Result<String, CliError> {
    use std::fmt::Write as _;
    let manifest = tamias_core::Manifest::load(dir)
        .map_err(|e| CliError(format!("{}: no readable tamias.yaml — {e}", dir.display())))?;
    let units = provider_units(&manifest);
    if units.is_empty() {
        return Err(CliError(format!(
            "{name}: provider declares no services — nothing to journal"
        )));
    }
    let mut out = String::new();
    for (_, unit) in &units {
        let full = format!("{unit}.service");
        let _ = writeln!(out, "── {full} ──");
        out.push_str(&run(&full, lines)?);
    }
    Ok(out)
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

    // --- T8-5 ops verbs ---------------------------------------------------

    /// A minimal HTTP/1.1 stub for the forge API + the hook-ping probe.
    /// Records each raw request (head + body); routes match
    /// `(METHOD, path-prefix)` in order, default 404.
    struct Stub {
        base: String,
        requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        /// Held so the accept loop outlives the test; never joined.
        #[allow(dead_code)]
        thread: std::thread::JoinHandle<()>,
    }

    impl Stub {
        /// `hooks_json` answers every `GET …/hooks` list call; POST →
        /// 201, PATCH/DELETE → 200/204. Any other method+path 404s.
        fn gh(hooks_json: &'static str) -> Self {
            Self::spawn(vec![
                ("GET", "/repos/", 200, hooks_json.to_string()),
                ("POST", "/repos/", 201, "{\"id\":42}".to_string()),
                ("PATCH", "/repos/", 200, "{\"id\":42}".to_string()),
                ("DELETE", "/repos/", 204, String::new()),
                ("POST", "/hook", 200, "pong".to_string()),
            ])
        }

        fn spawn(routes: Vec<(&'static str, &'static str, u16, String)>) -> Self {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let reqs = requests.clone();
            let routes: Vec<(String, String, u16, String)> = routes
                .into_iter()
                .map(|(m, p, s, b)| (m.to_string(), p.to_string(), s, b))
                .collect();
            let thread = std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    serve_conn(&mut stream, &reqs, &routes);
                }
            });
            Self {
                base,
                requests,
                thread,
            }
        }

        fn raw(&self) -> String {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n---\n")
        }
    }

    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        hay.windows(needle.len()).position(|w| w == needle)
    }

    fn serve_conn(
        stream: &mut std::net::TcpStream,
        reqs: &std::sync::Mutex<Vec<String>>,
        routes: &[(String, String, u16, String)],
    ) {
        use std::io::{Read, Write};
        let mut buf = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        let mut head_end = None;
        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if head_end.is_none() {
                        head_end = find(&buf, b"\r\n\r\n");
                    }
                    let Some(he) = head_end else { continue };
                    let head = String::from_utf8_lossy(&buf[..he]).to_string();
                    let len = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse().ok())
                        })
                        .unwrap_or(0);
                    if buf.len() < he + 4 + len {
                        continue;
                    }
                    let request = String::from_utf8_lossy(&buf).to_string();
                    reqs.lock().unwrap().push(request);
                    let line = head.lines().next().unwrap_or("").to_string();
                    let (status, body) = routes
                        .iter()
                        .find(|(m, p, _, _)| line.starts_with(&format!("{m} {p}")))
                        .map_or((404, "{}".into()), |(_, _, s, b)| (*s, b.clone()));
                    let res = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
                         content-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(res.as_bytes());
                    break;
                }
            }
        }
    }

    /// An `OpsEnv` sandbox whose env file carries the test secrets —
    /// `ops::secret` resolves env → keyring → this file; the file leg
    /// is what tests exercise (unique names avoid env collisions).
    fn ops_env(root: &Path) -> crate::ops::OpsEnv {
        let env = crate::ops::OpsEnv::under(root);
        std::fs::create_dir_all(&env.config_dir).unwrap();
        std::fs::write(
            env.env_file(),
            "T8S_GH_TOKEN=tok-test\nPEREGRIN_WH_9AIA_PEREGRIN=whsec-one\nPEREGRIN_WH_9AIA_OTHER=whsec-two\n",
        )
        .unwrap();
        env
    }

    #[test]
    fn hooks_dry_run_lists_targets_without_secrets_or_api() {
        let (tmp, dir) = bundle_dir();
        // No OpsEnv secrets, no api_base — dry-run must not need either.
        let env = crate::ops::OpsEnv::under(&tmp.path().join("nohome"));
        let out = hooks(&dir, &env, true, true, None, None).unwrap();
        assert!(out.contains("9aia/peregrin: would register https://ci.example.com/hook"));
        assert!(out.contains("9aia/other: would register https://ci.example.com/hook"));
        assert!(out.contains("events=[push]"));
        assert!(out.contains("secret=PEREGRIN_WH_9AIA_PEREGRIN"));
        assert!(!out.contains("ignored/group")); // gitlab forge stays data
        // unregister dry-run names the same targets
        let out = hooks(&dir, &env, false, true, None, None).unwrap();
        assert!(out.contains("would unregister"));
    }

    #[test]
    fn hooks_register_upserts_and_unregister_deletes() {
        let (tmp, dir) = bundle_dir();
        let env = ops_env(tmp.path());
        // Forge stub: no existing hooks → register POSTs creates.
        let stub = Stub::gh("[]");
        let api = stub.base.clone();
        let out = hooks(&dir, &env, true, false, Some(&api), Some("T8S_GH_TOKEN")).unwrap();
        assert!(out.contains("9aia/peregrin: created hook"), "{out}");
        assert!(out.contains("9aia/other: created hook"), "{out}");
        let raw = stub.raw();
        // one GET list + one POST per github repo; bodies carry the
        // hook URL, the resolved secret VALUE (never printed) and events.
        assert_eq!(raw.matches("GET /repos/").count(), 2, "{raw}");
        assert_eq!(raw.matches("POST /repos/").count(), 2, "{raw}");
        assert!(raw.contains("https://ci.example.com/hook"), "{raw}");
        assert!(raw.contains("whsec-one"), "{raw}");
        assert!(raw.contains("whsec-two"), "{raw}");
        assert!(raw.contains("\"push\""), "{raw}");
        // Report lines name repos + URLs but never secret values.
        assert!(!out.contains("whsec-one"), "{out}");
        assert!(!out.contains("whsec-two"), "{out}");

        // Idempotent: the forge now lists a hook at our URL → PATCH.
        let existing = r#"[{"id":7,"config":{"url":"https://ci.example.com/hook"}}]"#;
        let stub2 = Stub::gh(existing);
        let api2 = stub2.base.clone();
        let out = hooks(&dir, &env, true, false, Some(&api2), Some("T8S_GH_TOKEN")).unwrap();
        assert!(out.contains("updated hook 7"), "{out}");
        assert!(stub2.raw().contains("PATCH /repos/9aia/peregrin/hooks/7"));

        // Unregister: matching hook present → DELETE per repo.
        let stub3 = Stub::gh(existing);
        let api3 = stub3.base.clone();
        let out = hooks(&dir, &env, false, false, Some(&api3), Some("T8S_GH_TOKEN")).unwrap();
        assert!(out.contains("deleted hook 7"), "{out}");
        assert!(stub3.raw().contains("DELETE /repos/9aia/peregrin/hooks/7"));

        // Missing token secret → error before any API call.
        let err = hooks(
            &dir,
            &env,
            true,
            false,
            Some(&api3),
            Some("T8S_NO_SUCH_TOKEN"),
        )
        .unwrap_err();
        assert!(err.0.contains("T8S_NO_SUCH_TOKEN"), "{err}");
    }

    #[test]
    fn hooks_requires_github_hooks() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("plain");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tamias.yaml"), "name: p\n").unwrap();
        let env = crate::ops::OpsEnv::under(tmp.path());
        let err = hooks(&dir, &env, true, true, None, None).unwrap_err();
        assert!(err.0.contains("no provider"), "{err}");
    }

    #[test]
    fn health_pings_ingress_and_reports_json() {
        let tmp = tempfile::tempdir().unwrap();
        let stub = Stub::gh("[]");
        let addr = stub.base.trim_start_matches("http://").to_string();
        let dir = tmp.path().join("ci-gate@0.1.0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tamias.yaml"),
            format!(
                r#"name: ci-gate
version: "0.1.0"
type: peregrin-bundle
provider:
  name: ci-gate
  services:
    - name: campaign
      exec: "true"
  ingress:
    - public_url: https://ci.example.com
      listen: {addr}
      path: /hook
      hooks:
        - forge: github
          repo: 9aia/peregrin
          events: [push]
"#
            ),
        )
        .unwrap();
        let out = health(&dir, "ci-gate").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], true, "{out}");
        assert_eq!(v["probes"][0]["kind"], "hook-ping");
        assert!(stub.raw().contains("POST /hook"), "{}", stub.raw());
        assert!(
            stub.raw().to_lowercase().contains("x-github-event: ping"),
            "{}",
            stub.raw()
        );

        // Dead listener → ok:false, still JSON (no panic, no hang).
        let dir2 = tmp.path().join("dead");
        std::fs::create_dir_all(&dir2).unwrap();
        std::fs::write(
            dir2.join("tamias.yaml"),
            r#"name: dead
provider:
  name: dead
  services:
    - name: campaign
      exec: "true"
  ingress:
    - public_url: https://x.example.com
      listen: 127.0.0.1:1
      path: /hook
"#,
        )
        .unwrap();
        let out = health(&dir2, "dead").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], false, "{out}");
    }

    #[test]
    fn status_and_logs_report_units() {
        let (_tmp, dir) = bundle_dir();
        // Injected `systemctl --user`: is-active → active, is-enabled →
        // enabled (edition-2024 forbids env::set_var — no PATH fakery).
        let run = |args: &[&str]| -> Option<String> {
            Some(match args[0] {
                "is-active" => "active".to_string(),
                "is-enabled" => "enabled".to_string(),
                _ => "unknown".to_string(),
            })
        };
        let out = status_inner(&dir, "ci-gate", &run).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], true, "{out}");
        assert_eq!(v["units"][0]["unit"], "ci-gate-campaign.service");
        assert_eq!(v["units"][0]["active"], "active");
        assert_eq!(v["units"][0]["enabled"], "enabled");
        // systemd can't answer → "unknown", ok:false — still JSON.
        let out = status_inner(&dir, "ci-gate", &|_| None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], false, "{out}");
        assert_eq!(v["units"][0]["active"], "unknown");
        // Injected journal: marker + the unit it was asked for.
        let journal = |unit: &str, _lines: u32| -> Result<String, CliError> {
            Ok(format!("fake-journal {unit}\n"))
        };
        let out = logs_inner(&dir, "ci-gate", 50, &journal).unwrap();
        assert!(out.contains("ci-gate-campaign.service"), "{out}");
        assert!(out.contains("fake-journal"), "{out}");
    }
}
