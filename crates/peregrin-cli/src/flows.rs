//! `peregrin campaign flows …` — the live flow lifecycle client
//! (ADR-0093): talks to a running `campaign serve` daemon's
//! `/api/flows` routes on its listener socket, so `tamias deploy`
//! (via `provision`) and operators can add/remove/reload flows
//! without restarting the daemon.
//!
//! Target resolution: `--url` → `PEREGRIN_FLOWS_URL` →
//! `http://127.0.0.1:29173` (the `campaign serve --listen` default —
//! the same socket carries `/hook`, `/api/events/inject`, and
//! `/api/flows`). Token: `--token` → `PEREGRIN_TOKEN` env →
//! `PEREGRIN_TOKEN=` inside `~/.config/peregrin/env` — deploy-time
//! environments don't carry the unit's `EnvironmentFile`, so the
//! file leg is what makes `tamias install` work unattended.

use std::path::Path;

use serde_json::{Value, json};

use crate::CliError;
use crate::api::{self, ApiError, NodeTarget, urlencode};
use crate::ops::OpsEnv;

/// The `campaign serve --listen` default — where `/api/flows` lives
/// when the daemon runs with stock flags.
const DEFAULT_FLOWS_URL: &str = "http://127.0.0.1:29173";

/// `--url`/`PEREGRIN_FLOWS_URL`/default resolution.
#[must_use]
pub fn resolve_url(url: Option<&str>) -> String {
    url.map(str::to_string)
        .or_else(|| std::env::var("PEREGRIN_FLOWS_URL").ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_FLOWS_URL.to_string())
}

/// `PEREGRIN_TOKEN=` out of an env file — `KEY=value` lines,
/// `#` comments, optional surrounding quotes; first match wins.
#[must_use]
pub fn env_file_token(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "PEREGRIN_TOKEN" {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

/// `--token` → `PEREGRIN_TOKEN` env → `~/.config/peregrin/env`.
#[must_use]
pub fn resolve_token(token: Option<&str>) -> Option<String> {
    token
        .map(str::to_string)
        .or_else(|| std::env::var("PEREGRIN_TOKEN").ok())
        .filter(|t| !t.is_empty())
        .or_else(|| env_file_token(&OpsEnv::default().env_file()))
}

/// The resolved call target.
#[must_use]
pub fn target(url: Option<&str>, token: Option<&str>) -> NodeTarget {
    api::resolve_target(&resolve_url(url), resolve_token(token).as_deref())
}

fn call(
    target: &NodeTarget,
    method: &str,
    path: &str,
    body: Option<&Value>,
) -> Result<Value, CliError> {
    api::request(target, method, path, body).map_err(|e: ApiError| CliError(e.to_string()))
}

/// `flows list` — `GET /api/flows`, printed as the server's rows.
pub fn list(target: &NodeTarget) -> Result<(), CliError> {
    let value = call(target, "GET", "/api/flows", None)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_default()
    );
    Ok(())
}

/// `flows load <yaml> [--name]` — read the campaign doc and POST it
/// as `{name, yaml}`. The name defaults to the file stem (the same
/// fallback `campaign serve` uses for unnamed campaigns).
pub fn load(target: &NodeTarget, yaml: &Path, name: Option<&str>) -> Result<(), CliError> {
    let text = std::fs::read_to_string(yaml)
        .map_err(|e| CliError(format!("read {}: {e}", yaml.display())))?;
    let name = name
        .map(str::to_string)
        .or_else(|| {
            yaml.file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
        })
        .filter(|n| !n.is_empty())
        .ok_or_else(|| CliError(format!("cannot derive a flow name from {}", yaml.display())))?;
    let body = json!({ "name": name, "yaml": text });
    let value = call(target, "POST", "/api/flows", Some(&body))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_default()
    );
    Ok(())
}

/// `flows unload <name>` — `DELETE /api/flows/{name}` → `202
/// {draining}`; the flow's active run finishes in background.
pub fn unload(target: &NodeTarget, name: &str) -> Result<(), CliError> {
    let path = format!("/api/flows/{}", urlencode(name));
    let value = call(target, "DELETE", &path, None)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_default()
    );
    Ok(())
}

/// `flows reload <name>` — `POST /api/flows/{name}/reload` → drain
/// then respawn from the persisted source.
pub fn reload(target: &NodeTarget, name: &str) -> Result<(), CliError> {
    let path = format!("/api/flows/{}/reload", urlencode(name));
    let value = call(target, "POST", &path, None)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_default()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn env_file_token_parses_key_value_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("env");
        std::fs::write(
            &path,
            "# comment\nOTHER=x\nPEREGRIN_TOKEN=abc123\nPEREGRIN_TOKEN=ignored\n",
        )
        .unwrap();
        assert_eq!(env_file_token(&path).as_deref(), Some("abc123"));
    }

    #[test]
    fn env_file_token_handles_quotes_and_empties() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("env");
        std::fs::write(&path, "PEREGRIN_TOKEN=\"quoted tok\"\n").unwrap();
        assert_eq!(env_file_token(&path).as_deref(), Some("quoted tok"));

        std::fs::write(&path, "PEREGRIN_TOKEN=\nPEREGRIN_TOKEN='single'\n").unwrap();
        assert_eq!(env_file_token(&path).as_deref(), Some("single"));

        std::fs::write(&path, "PEREGRIN_TOKEN=   \n").unwrap();
        assert_eq!(env_file_token(&path), None);

        assert_eq!(env_file_token(&tmp.path().join("missing")), None);
    }

    #[test]
    fn resolve_url_prefers_the_flag() {
        assert_eq!(resolve_url(Some("http://x:1")), "http://x:1");
    }
}
