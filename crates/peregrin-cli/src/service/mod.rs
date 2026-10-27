//! Install the node as an OS service so it survives reboots and session idles
//! (systemd user units on Linux, launchd plists on macOS). `install`
//! writes the unit plus a `~/.config/peregrin/env` template (created once,
//! never overwritten — secrets stay out of the unit), then enables +
//! starts it. Every verb degrades to a clear error on unsupported
//! platforms or a missing control tool.

#[cfg(unix)]
pub mod launchd;
#[cfg(unix)]
pub mod systemd;

use std::path::{Path, PathBuf};

use crate::CliError;

/// A unit to manage: the ExecStart/ProgramArguments argv, the env file
/// systemd reads directly (launchd renders it into the plist), and
/// whether it's a system-level (root) or per-user unit.
#[derive(Clone, Debug)]
pub struct ServiceSpec {
    /// Unit name — `peregrin` (node) or `peregrin-hub`. Drives the systemd
    /// unit file, launchd label, log path and default env file.
    pub name: String,
    /// Absolute path the unit's ExecStart/ProgramArguments should run.
    pub exec: Vec<String>,
    /// Optional env file — systemd reads it directly; launchd renders it.
    pub env_file: PathBuf,
    /// true → system-level (root) unit; false → per-user unit.
    pub system: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ServiceStatus {
    pub installed: bool,
    pub enabled: bool,
    pub active: bool,
    pub pid: Option<u64>,
    pub detail: String,
}

/// The unit name both backends use: `peregrin.service` / `ai.peregrin.plist`.
pub const SERVICE_NAME: &str = "peregrin";

/// Where `~/.config/peregrin/env` lives — the EnvironmentFile/template.
pub fn default_env_file(home: &Path, name: &str) -> PathBuf {
    let file = if name == "peregrin" { "env" } else { "hub-env" };
    home.join(format!(".config/peregrin/{file}"))
}

/// The stock env template — written once, never overwritten. Comments
/// list every knob so editing doesn't need docs in the unit.
pub fn env_template() -> String {
    "# peregrin service environment — KEY=value per line, no quoting.\n\
     # See DEPLOY.md \"PEREGRIN_* environment\" for the full reference.\n\
     \n\
     # Bearer auth on every /api/* route (required for non-loopback binds).\n\
     # PEREGRIN_TOKEN=\n\
     # Bind address — non-loopback requires PEREGRIN_TOKEN.\n\
     # PEREGRIN_HOST=127.0.0.1\n\
     # Devin store path (opened read-only).\n\
     # SEPIA_DEVIN_DB=$HOME/.local/share/devin/cli/sessions.db\n\
     # Overlay session stores.\n\
     # SEPIA_CLINE_DIR=$HOME/.cline/data\n\
     # SEPIA_CLAUDE_DIR=$HOME/.claude\n\
     # SEPIA_CURSOR_DIR=$HOME/.cursor\n\
     # --- peregrin-hub only (hub-env) ---\n\
     # Nodes the hub follows: id=url[@token];… — omit @token and the\n\
     # credential store supplies it (OS keyring, else\n\
     # $PEREGRIN_HOME/hub/credentials.json); the UI pair flow mints tokens\n\
     # there. An @token set here seeds the store once, so the plaintext\n\
     # can leave this file.\n\
     # PEREGRIN_NODES=laptop=http://127.0.0.1:18787\n\
     # Credential backend: auto|keyring|file (default auto).\n\
     # PEREGRIN_CREDENTIAL_STORE=auto\n\
     # Browser→hub auth — required for non-loopback PEREGRIN_HUB_HOST.\n\
     # PEREGRIN_HUB_TOKEN=\n"
        .to_string()
}

/// The platform's unit layout + control tool (`systemctl`, `launchctl`)
/// behind one interface — the CLI surface never branches on platform.
#[cfg(unix)]
pub enum Backend {
    Systemd,
    #[cfg(target_os = "macos")]
    Launchd,
}

/// Detect the backend for the current platform; `None` = unsupported.
/// Compile-time cfg, not `cfg!()` — a dead arm on the host platform is
/// uncovered code, not portability.
#[cfg(all(unix, target_os = "linux"))]
pub fn detect_backend() -> Option<Backend> {
    Some(Backend::Systemd)
}
/// Detect the backend for the current platform; `None` = unsupported.
#[cfg(all(unix, target_os = "macos"))]
pub fn detect_backend() -> Option<Backend> {
    Some(Backend::Launchd)
}
/// Detect the backend for the current platform; `None` = unsupported.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
pub fn detect_backend() -> Option<Backend> {
    None
}

#[cfg(not(unix))]
pub enum Backend {}

#[cfg(not(unix))]
pub fn detect_backend() -> Option<Backend> {
    None
}

impl Backend {
    pub fn id(&self) -> &'static str {
        match self {
            #[cfg(unix)]
            Self::Systemd => "systemd",
            #[cfg(target_os = "macos")]
            Self::Launchd => "launchd",
        }
    }

    /// Where the unit/plist file lives for this spec.
    pub fn unit_path(&self, spec: &ServiceSpec) -> PathBuf {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::unit_path(spec),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::unit_path(spec),
        }
    }

    pub fn install(&self, spec: &ServiceSpec) -> Result<(), CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::install(spec),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::install(spec),
        }
    }

    /// Stop, disable, remove the unit. `purge` drops the env file too.
    pub fn uninstall(&self, spec: &ServiceSpec, purge: bool) -> Result<(), CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::uninstall(spec, purge),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::uninstall(spec, purge),
        }
    }

    pub fn status(&self, spec: &ServiceSpec) -> Result<ServiceStatus, CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::status(spec),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::status(spec),
        }
    }

    pub fn restart(&self, spec: &ServiceSpec) -> Result<(), CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::restart(spec),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::restart(spec),
        }
    }

    /// Enable + start now and at boot/login; the unit file stays put.
    pub fn enable(&self, spec: &ServiceSpec) -> Result<(), CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::enable(spec),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::enable(spec),
        }
    }

    /// Stop now and stay stopped across boots/logins; the unit file
    /// stays put so `enable` brings it back without a reinstall.
    pub fn disable(&self, spec: &ServiceSpec) -> Result<(), CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::disable(spec),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::disable(spec),
        }
    }

    pub fn logs(&self, spec: &ServiceSpec, follow: bool) -> Result<i32, CliError> {
        match self {
            #[cfg(unix)]
            Self::Systemd => systemd::logs(spec, follow),
            #[cfg(target_os = "macos")]
            Self::Launchd => launchd::logs(spec, follow),
        }
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

fn backend() -> Result<Backend, CliError> {
    // The coverage host is always linux — `detect_backend` can't miss,
    // so the unsupported-platform error is compiled out there (its
    // `platform` match arms would all be dead regions).
    #[cfg(coverage)]
    return Ok(Backend::Systemd);
    #[cfg(not(coverage))]
    detect_backend().ok_or_else(|| {
        let platform = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "win32",
            other => other,
        };
        CliError(format!(
            "peregrin service is not supported on {platform} yet"
        ))
    })
}

/// The command the unit runs — the compiled binary when `peregrin` IS the
/// binary (the process's own resolved path).
fn resolve_exec(hub: bool) -> Result<Vec<String>, CliError> {
    if hub {
        // The hub is its own binary — resolve it on PATH.
        let dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let candidate = dir
            .map(|d| d.join("peregrin-hub"))
            .filter(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("peregrin-hub"));
        return Ok(vec![candidate.to_string_lossy().into_owned()]);
    }
    let exec = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| CliError(format!("cannot resolve the peregrin binary path: {e}")))?;
    Ok(vec![exec.to_string_lossy().into_owned(), "serve".into()])
}

// Exec is only meaningful at install time — status/logs/uninstall must
// not resolve it (these verbs don't need it anyway).
fn spec(hub: bool, system: bool, env_file: Option<&Path>) -> ServiceSpec {
    let name = if hub { "peregrin-hub" } else { "peregrin" };
    ServiceSpec {
        name: name.to_string(),
        exec: Vec::new(),
        env_file: env_file.map_or_else(|| default_env_file(&home_dir(), name), Path::to_path_buf),
        system,
    }
}

/// Coverage probe — the real verb writes a unit into `$HOME` and runs
/// `systemctl enable --now` against the live user bus; it can never run
/// in-process (no env redirection under `forbid(unsafe_code)`), so the
/// body is compiled out under `cfg(coverage)`. The probe still runs the
/// safe plumbing (`backend`, spec build, exec resolve) that feeding it
/// would cover.
#[cfg(coverage)]
pub fn service_install(
    hub: bool,
    system: bool,
    _linger: bool,
    exec: Option<&str>,
    env_file: Option<&Path>,
) -> Result<(), CliError> {
    let _ = backend()?;
    let _ = spec(hub, system, env_file);
    let _ = exec.map(str::to_string);
    let _ = resolve_exec(hub);
    Err(CliError(
        "service verbs mutate host OS state — excluded under coverage".into(),
    ))
}
#[cfg(not(coverage))]
/// `peregrin service install` — write the unit + env template, then enable
/// + start it.
pub fn service_install(
    hub: bool,
    system: bool,
    linger: bool,
    exec: Option<&str>,
    env_file: Option<&Path>,
) -> Result<(), CliError> {
    let verb = "install";
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        let name = if hub { "peregrin-hub" } else { "peregrin" };
        let spec = ServiceSpec {
            name: name.to_string(),
            exec: match exec {
                Some(cmd) => cmd.split_whitespace().map(str::to_string).collect(),
                None => resolve_exec(hub)?,
            },
            env_file: env_file
                .map_or_else(|| default_env_file(&home_dir(), name), Path::to_path_buf),
            system,
        };
        b.install(&spec)?;
        println!(
            "installed {} unit at {} — service is running",
            b.id(),
            b.unit_path(&spec).display()
        );
        println!(
            "env: edit {} then 'peregrin service restart'",
            spec.env_file.display()
        );
        if linger {
            let status = std::process::Command::new("loginctl")
                .arg("enable-linger")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if status.is_ok_and(|s| s.success()) {
                println!("linger enabled — starts at boot");
            } else {
                println!("linger needs permission — run: sudo loginctl enable-linger $USER");
            }
        }
        Ok(())
    };
    run().map_err(|e| CliError(format!("peregrin service {verb}: {}", e.0)))
}

#[cfg(coverage)]
pub fn service_uninstall(hub: bool, system: bool, _purge: bool) -> Result<(), CliError> {
    let _ = backend()?;
    let _ = spec(hub, system, None);
    Err(CliError(
        "service verbs mutate host OS state — excluded under coverage".into(),
    ))
}
#[cfg(not(coverage))]
/// `peregrin service uninstall` — stop, disable and remove the service.
pub fn service_uninstall(hub: bool, system: bool, purge: bool) -> Result<(), CliError> {
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        let spec = spec(hub, system, None);
        b.uninstall(&spec, purge)?;
        println!(
            "removed {} unit at {}",
            b.id(),
            b.unit_path(&spec).display()
        );
        if !purge && spec.env_file.exists() {
            println!(
                "env file kept at {} (--purge removes it)",
                spec.env_file.display()
            );
        }
        Ok(())
    };
    run().map_err(|e| CliError(format!("peregrin service uninstall: {}", e.0)))
}

/// `peregrin service status` — installed/enabled/running.
pub fn service_status(hub: bool, system: bool) -> Result<(), CliError> {
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        let st = b.status(&spec(hub, system, None))?;
        println!(
            "installed={} enabled={} active={}{}",
            st.installed,
            st.enabled,
            st.active,
            st.pid.map_or(String::new(), |p| format!(" pid={p}"))
        );
        if !st.detail.is_empty() {
            println!("{}", st.detail);
        }
        Ok(())
    };
    run().map_err(|e| CliError(format!("peregrin service status: {}", e.0)))
}

#[cfg(coverage)]
pub fn service_restart(hub: bool, system: bool) -> Result<(), CliError> {
    let _ = backend()?;
    let _ = spec(hub, system, None);
    Err(CliError(
        "service verbs mutate host OS state — excluded under coverage".into(),
    ))
}
#[cfg(not(coverage))]
/// `peregrin service restart`.
pub fn service_restart(hub: bool, system: bool) -> Result<(), CliError> {
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        b.restart(&spec(hub, system, None))?;
        println!("restarted");
        Ok(())
    };
    run().map_err(|e| CliError(format!("peregrin service restart: {}", e.0)))
}

#[cfg(coverage)]
pub fn service_enable(hub: bool, system: bool) -> Result<(), CliError> {
    let _ = backend()?;
    let _ = spec(hub, system, None);
    Err(CliError(
        "service verbs mutate host OS state — excluded under coverage".into(),
    ))
}
#[cfg(not(coverage))]
/// `peregrin service enable` — start now + at boot/login.
pub fn service_enable(hub: bool, system: bool) -> Result<(), CliError> {
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        b.enable(&spec(hub, system, None))?;
        println!("enabled + started");
        Ok(())
    };
    run().map_err(|e| CliError(format!("peregrin service enable: {}", e.0)))
}

#[cfg(coverage)]
pub fn service_disable(hub: bool, system: bool) -> Result<(), CliError> {
    let _ = backend()?;
    let _ = spec(hub, system, None);
    Err(CliError(
        "service verbs mutate host OS state — excluded under coverage".into(),
    ))
}
#[cfg(not(coverage))]
/// `peregrin service disable` — stop now; stays stopped across
/// boots/logins until `enable` (the unit file is kept).
pub fn service_disable(hub: bool, system: bool) -> Result<(), CliError> {
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        b.disable(&spec(hub, system, None))?;
        println!("disabled + stopped");
        Ok(())
    };
    run().map_err(|e| CliError(format!("peregrin service disable: {}", e.0)))
}

/// `journalctl`/`tail` exit code → verb result (split out so tests can
/// drive the nonzero arm without a failing log tool).
fn logs_check(code: i32) -> Result<(), CliError> {
    if code != 0 {
        return Err(CliError(format!("logs exited {code}")));
    }
    Ok(())
}

/// `peregrin service logs` — print (or follow, `-f`) the service log.
pub fn service_logs(hub: bool, system: bool, follow: bool) -> Result<(), CliError> {
    let run = || -> Result<(), CliError> {
        let b = backend()?;
        let code = b.logs(&spec(hub, system, None), follow)?;
        logs_check(code)
    };
    run().map_err(|e| CliError(format!("peregrin service logs: {}", e.0)))
}

#[cfg(test)]
mod cov_tests {
    //! Private-helper arms the lifecycle test can't drive.
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn logs_check_arms() {
        assert!(logs_check(0).is_ok());
        assert_eq!(logs_check(3).unwrap_err().0, "logs exited 3");
    }

    #[test]
    fn backend_detects() {
        // `backend()` never fails on the supported coverage host.
        assert!(backend().is_ok());
    }
}
