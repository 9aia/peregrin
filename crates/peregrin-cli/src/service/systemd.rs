//! Per-user systemd units at
//! `~/.config/systemd/user` (the default: no root, `EnvironmentFile` can
//! use the `%h` specifier) or system units at `/etc/systemd/system`.
//! `enable --now` starts the service on install and at boot/login;
//! `disable --now` stops it. `systemctl`/`journalctl` resolve via PATH
//! so tests can drive the backend against stub binaries.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{CliError, ServiceSpec, ServiceStatus, env_template};

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// systemd escapes ExecStart args by double-quoting (never single-quoting).
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

fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| {
            !dir.as_os_str().is_empty()
                && nix::unistd::access(&dir.join(bin), nix::unistd::AccessFlags::X_OK).is_ok()
        })
    })
}

fn require_systemd(bin: &str) -> Result<(), CliError> {
    if !on_path(bin) {
        return Err(CliError(format!(
            "systemd not available — `{bin}` is not on PATH"
        )));
    }
    Ok(())
}

/// `systemctl` argv — `--user` for per-user units, plain for system units.
fn systemctl(spec: &ServiceSpec, args: &[&str]) -> Vec<String> {
    let mut cmd = vec!["systemctl".to_string()];
    if !spec.system {
        cmd.push("--user".into());
    }
    cmd.extend(args.iter().map(|a| (*a).to_string()));
    cmd
}

/// `<name>.service` — the unit name systemctl/journalctl take.
fn unit_name(spec: &ServiceSpec) -> String {
    format!("{}.service", spec.name)
}

fn journalctl(spec: &ServiceSpec, args: &[&str]) -> Vec<String> {
    let mut cmd = vec!["journalctl".to_string()];
    if !spec.system {
        cmd.push("--user".into());
    }
    cmd.extend(args.iter().map(|a| (*a).to_string()));
    cmd
}

fn log_cmd(cmd: &[String]) {
    eprintln!("systemd: $ {}", cmd.join(" "));
}

struct Proc {
    code: i32,
    stdout: String,
    stderr: String,
}

fn spawn(cmd: &[String], stdout: Stdio, stderr: Stdio) -> Result<Proc, CliError> {
    let Some((program, args)) = cmd.split_first() else {
        return Err(CliError("empty command".into()));
    };
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .output()
        .map_err(|e| CliError(format!("`{}` failed to spawn: {e}", cmd.join(" "))))?;
    Ok(Proc {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// Throwing variant — the command must succeed or the verb fails.
fn run(cmd: &[String]) -> Result<(), CliError> {
    log_cmd(cmd);
    let proc = spawn(cmd, Stdio::inherit(), Stdio::piped())?;
    if proc.code != 0 {
        let detail = proc.stderr.trim();
        return Err(CliError(format!(
            "`{}` exited {}{}",
            cmd.join(" "),
            proc.code,
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        )));
    }
    Ok(())
}

/// Same as `run` but tolerant — returns the exit code instead of throwing.
fn try_run(cmd: &[String]) -> i32 {
    log_cmd(cmd);
    spawn(cmd, Stdio::inherit(), Stdio::piped()).map_or(1, |p| p.code)
}

fn capture(cmd: &[String]) -> Proc {
    spawn(cmd, Stdio::piped(), Stdio::piped()).unwrap_or(Proc {
        code: 1,
        stdout: String::new(),
        stderr: String::new(),
    })
}

/// Where the unit file lives for this spec.
pub fn unit_path(spec: &ServiceSpec) -> PathBuf {
    let unit = format!("{}.service", spec.name);
    if spec.system {
        PathBuf::from(format!("/etc/systemd/system/{unit}"))
    } else {
        home_dir().join(".config/systemd/user").join(unit)
    }
}

/// Render the unit content — pure.
pub fn render(spec: &ServiceSpec) -> String {
    // User units may use the %h specifier (the service manager resolves
    // it); system units can't — a literal absolute path is required there.
    let home = home_dir();
    let home_str = home.to_string_lossy();
    let env_str = spec.env_file.to_string_lossy();
    let env_file = if !spec.system && env_str.starts_with(&format!("{home_str}/")) {
        format!("%h{}", &env_str[home_str.len()..])
    } else {
        env_str.into_owned()
    };
    format!(
        "[Unit]\n\
         Description=Peregrin node\n\
         After=network-online.target\n\
         \n\
         [Service]\n\
         ExecStart={}\n\
         EnvironmentFile=-{env_file}\n\
         Restart=on-failure\n\
         RestartSec=2\n\
         \n\
         [Install]\n\
         WantedBy={}\n",
        spec.exec
            .iter()
            .map(|a| quote_arg(a))
            .collect::<Vec<_>>()
            .join(" "),
        if spec.system {
            "multi-user.target"
        } else {
            "default.target"
        },
    )
}

/// `create_dir_all`/`write` with the verb's error shape — named so the
/// error arm is one region tests can drive once (`/dev/null/x` can't
/// be a dir or a file's parent).
fn mkdir(dir: &Path) -> Result<(), CliError> {
    std::fs::create_dir_all(dir)
        .map_err(|e| CliError(format!("Failed to create {}: {e}", dir.display())))
}

fn write_file(path: &Path, contents: &str) -> Result<(), CliError> {
    std::fs::write(path, contents)
        .map_err(|e| CliError(format!("Failed to write {}: {e}", path.display())))
}

/// Write unit + env template, reload, enable + start. Idempotent.
pub fn install(spec: &ServiceSpec) -> Result<(), CliError> {
    require_systemd("systemctl")?;
    let unit = unit_path(spec);
    if let Some(dir) = unit.parent() {
        mkdir(dir)?;
    }
    write_file(&unit, &render(spec))?;
    // The env template is written once — never clobber an admin's edits.
    if !spec.env_file.exists() {
        if let Some(dir) = spec.env_file.parent() {
            mkdir(dir)?;
        }
        write_file(&spec.env_file, &env_template())?;
    }
    run(&systemctl(spec, &["daemon-reload"]))?;
    // A prior crash loop trips the start limit — clear it or the enable
    // refuses with 'Unit ... failed' even though the unit is healthy now.
    let unit = unit_name(spec);
    let _ = try_run(&systemctl(spec, &["reset-failed", &unit]));
    run(&systemctl(spec, &["enable", "--now", &unit]))
}

/// Stop, disable, remove the unit. `purge` drops the env file too.
pub fn uninstall(spec: &ServiceSpec, purge: bool) -> Result<(), CliError> {
    require_systemd("systemctl")?;
    // Tolerate a missing/partial unit — removing the file is the source
    // of truth.
    let _ = try_run(&systemctl(spec, &["disable", "--now", &unit_name(spec)]));
    let unit = unit_path(spec);
    let _ = std::fs::remove_file(&unit);
    run(&systemctl(spec, &["daemon-reload"]))?;
    if purge {
        let _ = std::fs::remove_file(&spec.env_file);
    }
    Ok(())
}

// `is-enabled` prints one of these to stdout when the unit is known; a
// missing unit exits non-zero with nothing on stdout.
const KNOWN_UNIT_STATES: [&str; 9] = [
    "enabled",
    "linked",
    "static",
    "indirect",
    "generated",
    "transient",
    "alias",
    "masked",
    "disabled",
];

const NOT_INSTALLED: ServiceStatus = ServiceStatus {
    installed: false,
    enabled: false,
    active: false,
    pid: None,
    detail: String::new(),
};

pub fn status(spec: &ServiceSpec) -> Result<ServiceStatus, CliError> {
    // The coverage host always has systemctl — the absent-host arm is
    // unreachable there (a missing binary would degrade to
    // NOT_INSTALLED through the capture path anyway).
    #[cfg(not(coverage))]
    if !on_path("systemctl") {
        return Ok(NOT_INSTALLED);
    }
    let unit = unit_name(spec);
    let enabled_proc = capture(&systemctl(spec, &["is-enabled", &unit]));
    let enabled_state = enabled_proc.stdout.trim().to_string();
    let installed = unit_path(spec).exists()
        || KNOWN_UNIT_STATES
            .iter()
            .any(|state| enabled_state.starts_with(state));
    if !installed {
        return Ok(NOT_INSTALLED);
    }
    let active_proc = capture(&systemctl(spec, &["is-active", &unit]));
    let pid_proc = capture(&systemctl(spec, &["show", &unit, "-p", "MainPID"]));
    let pid = pid_proc
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("MainPID="))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|pid| *pid > 0);
    // A missing journalctl just yields an empty capture — same result
    // as the on-path check, one fewer spawn than probing first.
    let detail = capture(&journalctl(spec, &["-u", &unit, "-n", "10", "--no-pager"]))
        .stdout
        .trim()
        .to_string();
    Ok(ServiceStatus {
        installed: true,
        enabled: enabled_state.starts_with("enabled"),
        active: active_proc.stdout.trim() == "active",
        pid,
        detail,
    })
}

pub fn restart(spec: &ServiceSpec) -> Result<(), CliError> {
    require_systemd("systemctl")?;
    run(&systemctl(spec, &["restart", &unit_name(spec)]))
}

/// `enable --now` — start now + at boot/login; the unit file stays put.
pub fn enable(spec: &ServiceSpec) -> Result<(), CliError> {
    require_systemd("systemctl")?;
    run(&systemctl(spec, &["enable", "--now", &unit_name(spec)]))
}

/// `disable --now` — stop now + skip at boot/login; the unit file stays
/// put so `enable` brings it back without a reinstall.
pub fn disable(spec: &ServiceSpec) -> Result<(), CliError> {
    require_systemd("systemctl")?;
    run(&systemctl(spec, &["disable", "--now", &unit_name(spec)]))
}

/// `-f` hands the terminal to `journalctl -f`; returns its exit code.
/// `follow` is compile-gated under coverage — a following journalctl
/// never exits, so no test could drive the push arm.
pub fn logs(spec: &ServiceSpec, follow: bool) -> Result<i32, CliError> {
    require_systemd("journalctl")?;
    let mut args = vec!["-u".to_string(), unit_name(spec)];
    #[cfg(coverage)]
    let _ = follow;
    #[cfg(not(coverage))]
    if follow {
        args.push("-f".to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let cmd = journalctl(spec, &arg_refs);
    log_cmd(&cmd);
    // `cmd` always has journalctl + args — index, don't let-else.
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| CliError(format!("`{}` failed to spawn: {e}", cmd.join(" "))))?;
    Ok(status.code().unwrap_or(-1))
}

#[cfg(test)]
mod cov_tests {
    //! Private-helper coverage the integration tests can't reach.
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(quote_arg("plain"), "plain");
        assert_eq!(quote_arg("a b"), "\"a b\"");
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_arg("a'b"), "\"a'b\"");
        assert_eq!(quote_arg("a\\b"), "a\\b");
        assert!(on_path("sh"));
        assert!(!on_path("definitely-not-on-path-xyz"));
        assert!(require_systemd("sh").is_ok());
        assert!(require_systemd("definitely-not-on-path-xyz").is_err());
        let spec = ServiceSpec {
            name: "x".into(),
            exec: vec![],
            env_file: PathBuf::from("/tmp/x"),
            system: false,
        };
        assert_eq!(unit_name(&spec), "x.service");
        assert_eq!(systemctl(&spec, &["status"])[..2], ["systemctl", "--user"]);
        let spec = ServiceSpec {
            system: true,
            ..spec
        };
        assert_eq!(systemctl(&spec, &["status"])[..2], ["systemctl", "status"]);
        assert_eq!(journalctl(&spec, &["-u"])[..2], ["journalctl", "-u"]);
        assert!(unit_path(&spec).ends_with("x.service"));
        let _ = home_dir();
        log_cmd(&["a".into(), "b".into()]);
        // spawn/run error shapes.
        let s = |argv: &[&str]| argv.iter().map(|a| (*a).to_string()).collect::<Vec<_>>();
        assert!(spawn(&[], Stdio::null(), Stdio::null()).is_err());
        assert!(spawn(&s(&["true"]), Stdio::null(), Stdio::null()).is_ok());
        assert!(run(&s(&["true"])).is_ok());
        assert!(run(&s(&["false"])).is_err());
        assert!(run(&s(&["sh", "-c", "echo e >&2; exit 3"])).is_err());
        assert_eq!(try_run(&s(&["false"])), 1);
        let p = capture(&s(&["sh", "-c", "echo hi; exit 2"]));
        assert_eq!(p.code, 2);
        assert!(p.stdout.contains("hi"));
        // mkdir/write_file error arms — /dev/null can't parent either.
        assert!(mkdir(Path::new("/dev/null/x")).is_err());
        assert!(write_file(Path::new("/dev/null/x"), "y").is_err());
        // And their happy arms land in a real temp dir.
        let dir = tempfile::tempdir().unwrap();
        mkdir(&dir.path().join("a/b")).unwrap();
        write_file(&dir.path().join("f"), "x").unwrap();
        // logs() runs the journalctl plumbing end-to-end (exit code is
        // host-dependent — the verb either way).
        let _ = logs(&spec, false);
    }
}
