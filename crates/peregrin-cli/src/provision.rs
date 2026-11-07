//! `peregrin provision …` — the tamias **content-type provider** entrypoint.
//!
//! tamias owns distribution/provisioning/deploy as *generic* verbs; it
//! shells `peregrin provision {install,deploy,uninstall} --dir <dir>
//! --name <name> [--config <path>]` for the peregrin-specific bits (the
//! node config + the service unit) and never learns peregrin's schema.
//!
//! - `install`   — write `<name>.yaml` (a node config `include:`ing the
//!   bundle's `main.yaml`) from an installed bundle dir.
//! - `deploy`    — write the per-user systemd unit (`peregrin serve`);
//!   the operator enables it (`tamias service enable <name>`).
//! - `uninstall` — remove the node config + the unit.

use std::path::{Path, PathBuf};

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

/// `install` — write a node config that `include:`s the bundle's main.yaml.
///
/// # Errors
/// Fails when the bundle dir has no `main.yaml` or the write fails.
pub fn install(dir: &Path, name: &str, config: Option<&Path>) -> Result<String, CliError> {
    let main = ["main.yaml", "main.yml"]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| CliError(format!("bundle dir {} has no main.yaml", dir.display())))?;
    let path = config_path(name, config);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CliError(e.to_string()))?;
    }
    let doc = format!(
        "name: {name}\n\
         annotations:\n  generated_by: \"tamias install (peregrin provider)\"\n\
         include:\n  - {}\n\
         config:\n  name: {name}\n",
        main.display()
    );
    std::fs::write(&path, doc).map_err(|e| CliError(e.to_string()))?;
    Ok(format!("wrote node config {}", path.display()))
}

/// `deploy` — write the per-user systemd unit (does not enable it).
///
/// # Errors
/// Fails when the unit can't be written.
pub fn deploy(_dir: &Path, name: &str, config: Option<&Path>) -> Result<String, CliError> {
    let unit_dir = peregrin_systemd::default_unit_dir();
    let env_file = config.map_or_else(|| config_dir().join("env"), Path::to_path_buf);
    let exec = vec!["peregrin".to_string(), "serve".to_string()];
    let path = peregrin_systemd::write_unit(
        &unit_dir,
        name,
        &exec,
        &env_file,
        &format!("peregrin {name}"),
    )
    .map_err(|e| CliError(e.to_string()))?;
    Ok(format!(
        "wrote unit {} (enable: tamias service enable {name})",
        path.display()
    ))
}

/// `uninstall` — remove the node config + the unit.
///
/// # Errors
/// Best-effort — always returns Ok unless the name is empty.
pub fn uninstall(name: &str, config: Option<&Path>) -> Result<String, CliError> {
    let path = config_path(name, config);
    if path.is_file() {
        let _ = std::fs::remove_file(&path);
    }
    let _ = peregrin_systemd::remove(&peregrin_systemd::default_unit_dir(), name);
    Ok(format!("removed config + unit for {name}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

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
}
