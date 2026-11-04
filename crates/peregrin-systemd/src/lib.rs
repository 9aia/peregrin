//! peregrin-systemd — the systemd impl of the `peregrin-service`
//! port: per-user units at `~/.config/systemd/user` (no root needed;
//! `EnvironmentFile` keeps secrets out of the unit). Used by the P6
//! ops verbs — `peregrin create --template` *writes* a unit without
//! enabling it (the operator's `peregrin service enable` is the
//! separate step), `peregrin destroy service` disables + removes it.
//!
//! `systemctl` resolves via PATH and every control call is separate
//! from the filesystem ops, so tests can point `unit_dir` at a
//! tempdir and drive the write/remove paths without touching the
//! user bus (mirroring `bupha-systemd`'s `SystemdService` shape).

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `~/.config/systemd/user` — the default per-user unit dir.
#[must_use]
pub fn default_unit_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("systemd/user")
}

/// `<unit_dir>/<name>.service` — the unit file's path.
#[must_use]
pub fn unit_path(unit_dir: &Path, name: &str) -> PathBuf {
    unit_dir.join(format!("{name}.service"))
}

/// systemd escapes ExecStart args by double-quoting (never
/// single-quoting) — same rule as `peregrin-cli`'s service module.
fn quote_arg(arg: &str) -> String {
    if arg
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '\'')
    {
        format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        arg.to_string()
    }
}

/// Render the unit — pure. `EnvironmentFile=-` tolerates a missing env
/// file; `WantedBy=default.target` is the user-unit target.
#[must_use]
pub fn render(exec: &[String], env_file: &Path, description: &str) -> String {
    format!(
        "[Unit]\n\
         Description={description}\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         EnvironmentFile=-{}\n\
         ExecStart={}\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        env_file.display(),
        exec.iter()
            .map(|a| quote_arg(a))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Write the unit file only — no `daemon-reload`, no enable. `create
/// --template` calls this; enabling stays the operator's explicit
/// `peregrin service enable --unit <name>` step.
///
/// # Errors
/// `io::Error` when the unit dir can't be created or the file written.
pub fn write_unit(
    unit_dir: &Path,
    name: &str,
    exec: &[String],
    env_file: &Path,
    description: &str,
) -> io::Result<PathBuf> {
    std::fs::create_dir_all(unit_dir)?;
    let path = unit_path(unit_dir, name);
    std::fs::write(&path, render(exec, env_file, description))?;
    Ok(path)
}

fn run(bin: &str, args: &[&str]) -> io::Result<()> {
    let status = Command::new(bin).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{bin} {args:?} -> {status}")))
    }
}

/// `systemctl --user daemon-reload` — tolerates a missing bus.
pub fn daemon_reload() {
    let _ = run("systemctl", &["--user", "daemon-reload"]);
}

/// `daemon-reload` + `enable --now` — start now and at login.
///
/// # Errors
/// `io::Error` when `systemctl` fails.
pub fn enable(name: &str) -> io::Result<()> {
    let unit = format!("{name}.service");
    daemon_reload();
    run("systemctl", &["--user", "enable", "--now", &unit])
}

/// `disable --now` — stop now, stay stopped across logins; the unit
/// file is kept (the `peregrin-service` port semantics).
///
/// # Errors
/// `io::Error` when `systemctl` fails.
pub fn disable(name: &str) -> io::Result<()> {
    let unit = format!("{name}.service");
    run("systemctl", &["--user", "disable", "--now", &unit])
}

/// Disable (best-effort — an absent or never-enabled unit still loses
/// its file), remove the unit file, reload. Returns the removed unit's
/// path. The unit file is the source of truth: a missing bus or unit
/// doesn't block teardown.
///
/// # Errors
/// `io::Error` only when the file removal itself fails.
pub fn remove(unit_dir: &Path, name: &str) -> io::Result<PathBuf> {
    let _ = disable(name);
    let path = unit_path(unit_dir, name);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    daemon_reload();
    Ok(path)
}

/// `enabled=<state> active=<state>` for status surfaces — probe
/// failures report `unknown` rather than throwing.
#[must_use]
pub fn status(name: &str) -> String {
    let unit = format!("{name}.service");
    let probe = |args: &[&str]| -> String {
        Command::new("systemctl").args(args).output().map_or_else(
            |_| "unknown".into(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        )
    };
    format!(
        "enabled={} active={}",
        probe(&["--user", "is-enabled", &unit]),
        probe(&["--user", "is-active", &unit])
    )
}

/// `systemctl --user restart` — bounce the unit.
///
/// # Errors
/// `io::Error` when `systemctl` fails.
pub fn restart(name: &str) -> io::Result<()> {
    let unit = format!("{name}.service");
    run("systemctl", &["--user", "restart", &unit])
}

/// systemd impl of the [`peregrin_service::Service`] port — one named
/// unit. `unit_dir` is injectable so tests manage units in a tempdir.
pub struct SystemdService {
    /// Directory holding `<name>.service`.
    pub unit_dir: PathBuf,
    /// Unit name (without the `.service` suffix).
    pub name: String,
    /// ExecStart argv.
    pub exec: Vec<String>,
    /// `EnvironmentFile=-` path.
    pub env_file: PathBuf,
    /// `[Unit] Description=` text.
    pub description: String,
}

impl SystemdService {
    /// A service under the default per-user unit dir.
    #[must_use]
    pub fn user(name: &str) -> Self {
        Self {
            unit_dir: default_unit_dir(),
            name: name.to_string(),
            exec: Vec::new(),
            env_file: PathBuf::new(),
            description: format!("Peregrin campaign {name}"),
        }
    }

    /// Write the unit file (no enable) — see [`write_unit`].
    ///
    /// # Errors
    /// `io::Error` on filesystem failure.
    pub fn write(&self) -> io::Result<PathBuf> {
        write_unit(
            &self.unit_dir,
            &self.name,
            &self.exec,
            &self.env_file,
            &self.description,
        )
    }

    /// Disable + remove the unit — see [`remove`].
    ///
    /// # Errors
    /// `io::Error` when the file removal fails.
    pub fn destroy(&self) -> io::Result<PathBuf> {
        remove(&self.unit_dir, &self.name)
    }
}

impl peregrin_service::Service for SystemdService {
    fn status(&self) -> io::Result<String> {
        Ok(status(&self.name))
    }

    fn restart(&self) -> io::Result<()> {
        restart(&self.name)
    }

    /// Enable + start now and at boot/login.
    fn enable(&self) -> io::Result<()> {
        enable(&self.name)
    }

    /// Stop now and stay stopped across boots/logins — the port's
    /// contract keeps the unit file (`destroy` removes it).
    fn disable(&self) -> io::Result<()> {
        disable(&self.name)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use peregrin_service::Service;
    use tempfile::TempDir;

    fn exec() -> Vec<String> {
        vec!["/usr/bin/peregrin".into(), "campaign".into(), "run".into()]
    }

    #[test]
    fn paths_and_render() {
        let dir = PathBuf::from("/units");
        assert_eq!(unit_path(&dir, "ci-gate"), dir.join("ci-gate.service"));
        let text = render(
            &[
                "peregrin".into(),
                "campaign run".into(),
                "say \"hi\"".into(),
            ],
            Path::new("/cfg/env"),
            "Peregrin campaign x",
        );
        assert!(text.contains("EnvironmentFile=-/cfg/env"));
        assert!(text.contains("ExecStart=peregrin \"campaign run\" \"say \\\"hi\\\"\""));
        assert!(text.contains("Description=Peregrin campaign x"));
        assert!(text.contains("WantedBy=default.target"));
        assert_eq!(quote_arg("plain"), "plain");
        assert_eq!(quote_arg("a'b"), "\"a'b\"");
    }

    #[test]
    fn write_and_remove_unit() {
        let tmp = TempDir::new().unwrap();
        let units = tmp.path().join("units");
        let svc = SystemdService {
            unit_dir: units.clone(),
            ..SystemdService::user("covtest")
        };
        // write → file exists with the rendered body.
        let path = svc.write().unwrap();
        assert!(path.exists());
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("Description=Peregrin campaign covtest"));
        // destroy → file gone, returns the path it removed.
        let removed = svc.destroy().unwrap();
        assert_eq!(removed, path);
        assert!(!path.exists());
        // remove tolerates a missing unit entirely.
        let again = remove(&units, "ghost").unwrap();
        assert!(!again.exists());
    }

    #[test]
    fn write_error_arm() {
        // `/dev/null/x` can't be a directory.
        let bad = Path::new("/dev/null/x");
        assert!(write_unit(bad, "x", &exec(), Path::new("/e"), "d").is_err());
    }

    #[test]
    fn remove_error_arm() {
        let tmp = TempDir::new().unwrap();
        let units = tmp.path().join("units");
        // A *directory* named x.service — remove_file refuses.
        std::fs::create_dir_all(units.join("x.service")).unwrap();
        assert!(remove(&units, "x").is_err());
    }

    #[test]
    fn control_verbs_run_or_error() {
        // Host-dependent — the verbs either drive systemctl or return
        // its error; both are covered paths.
        let _ = enable("peregrin-covtest-nonexistent");
        let _ = disable("peregrin-covtest-nonexistent");
        let _ = restart("peregrin-covtest-nonexistent");
        let out = status("peregrin-covtest-nonexistent");
        assert!(out.starts_with("enabled="));
        // `run` failure arm — a binary that doesn't exist.
        assert!(run("peregrin-covtest-no-such-bin", &["x"]).is_err());
        assert!(run("true", &[]).is_ok());
        daemon_reload();
    }

    #[test]
    fn service_trait_impl() {
        let svc = SystemdService::user("covtest");
        let text = Service::status(&svc).unwrap();
        assert!(text.starts_with("enabled="));
        let _ = Service::restart(&svc);
        let _ = Service::enable(&svc);
        let _ = Service::disable(&svc);
    }

    #[test]
    fn default_unit_dir_under_home() {
        let dir = default_unit_dir();
        assert!(dir.ends_with("systemd/user"));
    }
}
