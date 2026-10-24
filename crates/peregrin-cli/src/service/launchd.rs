//! One `ai.peregrin` job: a
//! LaunchAgent under `~/Library/LaunchAgents` for `--user` (the only
//! sensible default for a dev tool) or a LaunchDaemon under
//! `/Library/LaunchDaemons` for `--system`. Unlike systemd, launchd has
//! no EnvironmentFile directive, so the env file is rendered into the
//! plist's `EnvironmentVariables` dict at install time — edit the file,
//! re-run `peregrin service install` to apply.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(not(coverage))]
use super::env_template;
use super::{CliError, ServiceSpec, ServiceStatus};

/// `ai.peregrin` for the node, `ai.peregrin-hub` for the hub.
fn label(spec: &ServiceSpec) -> String {
    format!("ai.{}", spec.name.replace('-', "."))
}
/// `status` shows the last N log lines; `logs` prints the last N.
const STATUS_LOG_LINES: usize = 10;
const LOG_LINES: usize = 50;

fn uid() -> u32 {
    nix::unistd::getuid().as_raw()
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// The launchd domain a spec loads into: `gui/<uid>` or `system`.
fn domain(spec: &ServiceSpec) -> String {
    if spec.system {
        "system".into()
    } else {
        format!("gui/{}", uid())
    }
}

/// The service endpoint `print`/`kickstart` take: `gui/<uid>/ai.peregrin`.
fn service_target(spec: &ServiceSpec) -> String {
    format!("{}/{}", domain(spec), label(spec))
}

fn log_path(spec: &ServiceSpec) -> PathBuf {
    if spec.system {
        PathBuf::from(format!("/var/log/{}.log", spec.name))
    } else {
        home_dir().join(format!("Library/Logs/{}.log", spec.name))
    }
}

/// Where the plist file lives for this spec.
pub fn unit_path(spec: &ServiceSpec) -> PathBuf {
    if spec.system {
        PathBuf::from(format!("/Library/LaunchDaemons/{}.plist", label(spec)))
    } else {
        home_dir()
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", label(spec)))
    }
}

// MARK: env file → plist

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse `KEY=value` lines out of the env file — blanks and `#` comments
/// are ignored, a missing file yields no vars (install still renders a
/// valid plist). Duplicate keys collapse to the last value.
fn parse_env_file(path: &Path) -> Vec<(String, String)> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut entries: Vec<(String, String)> = Vec::new();
    for line in raw.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(eq) = trimmed.find('=') else {
            continue;
        };
        if eq == 0 {
            continue;
        }
        let key = trimmed[..eq].trim();
        if !valid_env_key(key) {
            continue;
        }
        let value = trimmed[eq + 1..].trim().to_string();
        // Duplicate keys collapse to the last value, position stable.
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => entries.push((key.to_string(), value)),
        }
    }
    entries
}

/// Render the plist content — pure.
pub fn render(spec: &ServiceSpec) -> String {
    let env = parse_env_file(&spec.env_file);
    let args = spec
        .exec
        .iter()
        .map(|arg| format!("\t\t<string>{}</string>", xml_escape(arg)))
        .collect::<Vec<_>>()
        .join("\n");
    let env_block = if env.is_empty() {
        String::new()
    } else {
        let pairs = env
            .iter()
            .map(|(key, value)| {
                format!(
                    "\t\t<key>{}</key>\n\t\t<string>{}</string>",
                    xml_escape(key),
                    xml_escape(value)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("\t<key>EnvironmentVariables</key>\n\t<dict>\n{pairs}\n\t</dict>\n")
    };
    let log = xml_escape(&log_path(spec).to_string_lossy());
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         {args}\n\
         \t</array>\n\
         {env_block}\t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         \t<key>KeepAlive</key>\n\
         \t<dict>\n\
         \t\t<key>SuccessfulExit</key>\n\
         \t\t<false/>\n\
         \t</dict>\n\
         \t<key>StandardOutPath</key>\n\
         \t<string>{log}</string>\n\
         \t<key>StandardErrorPath</key>\n\
         \t<string>{log}</string>\n\
         </dict>\n\
         </plist>\n",
        label(spec)
    )
}

// MARK: launchctl

/// `bin` resolves on PATH — the install preflight's whole question.
fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| {
            !dir.as_os_str().is_empty()
                && nix::unistd::access(&dir.join(bin), nix::unistd::AccessFlags::X_OK).is_ok()
        })
    })
}

/// `bin` is a parameter so tests can drive both arms with real
/// binaries (`sh`) — `launchctl` itself never exists on a linux host.
fn require_bin(bin: &str) -> Result<(), CliError> {
    if !on_path(bin) {
        return Err(CliError(format!(
            "launchd: `{bin}` not found on PATH — is this macOS?"
        )));
    }
    Ok(())
}

fn require_launchctl() -> Result<(), CliError> {
    require_bin("launchctl")
}

struct Proc {
    code: i32,
    stdout: String,
    stderr: String,
}

/// One launchctl invocation, echoed to stdout before it runs.
fn spawn(argv: &[&str]) -> Proc {
    println!("launchd: $ {}", argv.join(" "));
    let Some((program, args)) = argv.split_first() else {
        return Proc {
            code: 1,
            stdout: String::new(),
            stderr: "empty command".into(),
        };
    };
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(out) => Proc {
            code: out.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => Proc {
            code: 1,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

/// Throwing variant — the command must succeed or the verb fails.
fn run(argv: &[&str]) -> Result<(), CliError> {
    let out = spawn(argv);
    if out.code != 0 {
        let detail = {
            let stderr = out.stderr.trim();
            if stderr.is_empty() {
                out.stdout.trim()
            } else {
                stderr
            }
        };
        return Err(CliError(format!(
            "launchd: `{}` exited {}: {}",
            argv.join(" "),
            out.code,
            detail
        )));
    }
    Ok(())
}

/// Whether launchd currently has the job loaded (`print` probes it).
fn loaded(spec: &ServiceSpec) -> bool {
    spawn(&["launchctl", "print", &service_target(spec)]).code == 0
}

/// Last `count` lines of a log file — "" when the file isn't there.
fn tail_lines(path: &Path, count: usize) -> String {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let mut lines: Vec<&str> = raw.split('\n').collect();
    while lines.last() == Some(&"") {
        lines.pop();
    }
    lines
        .iter()
        .skip(lines.len().saturating_sub(count))
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Coverage probe — `launchctl` can never exist on the linux coverage
/// host, so the real verb body is compiled out under `cfg(coverage)`;
/// the probe still runs the on-PATH check (the linux answer: absent).
#[cfg(coverage)]
pub fn install(_spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()
}
#[cfg(not(coverage))]
/// Write the env template (first install only — the file is user state,
/// a re-install only re-renders it into the plist), write the plist,
/// then bootstrap into the gui/system domain. Re-installing over a
/// loaded job bootouts first so the new plist actually takes.
pub fn install(spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()?;
    let plist = unit_path(spec);
    for dir in [
        plist.parent(),
        log_path(spec).parent(),
        spec.env_file.parent(),
    ]
    .into_iter()
    .flatten()
    {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError(format!("Failed to create {}: {e}", dir.display())))?;
    }
    if !spec.env_file.exists() {
        std::fs::write(&spec.env_file, env_template())
            .map_err(|e| CliError(format!("Failed to write {}: {e}", spec.env_file.display())))?;
    }
    std::fs::write(&plist, render(spec))
        .map_err(|e| CliError(format!("Failed to write {}: {e}", plist.display())))?;
    if loaded(spec) {
        let plist_str = plist.to_string_lossy().into_owned();
        let _ = spawn(&["launchctl", "bootout", &domain(spec), &plist_str]);
    }
    run(&[
        "launchctl",
        "bootstrap",
        &domain(spec),
        &plist.to_string_lossy(),
    ])
}

/// bootout is best-effort — "not loaded" isn't a failure to remove files.
pub fn uninstall(spec: &ServiceSpec, purge: bool) -> Result<(), CliError> {
    // `launchctl` never exists on the coverage host — the block would
    // be unreachable lines there.
    #[cfg(not(coverage))]
    if on_path("launchctl") {
        let unit = unit_path(spec).to_string_lossy().into_owned();
        let _ = spawn(&["launchctl", "bootout", &domain(spec), &unit]);
    }
    let _ = std::fs::remove_file(unit_path(spec));
    if purge {
        let _ = std::fs::remove_file(&spec.env_file);
    }
    Ok(())
}

#[cfg(coverage)]
pub fn status(spec: &ServiceSpec) -> Result<ServiceStatus, CliError> {
    let enabled = unit_path(spec).exists();
    let detail = tail_lines(&log_path(spec), STATUS_LOG_LINES);
    let _ = on_path("launchctl");
    Ok(ServiceStatus {
        installed: false,
        enabled,
        active: false,
        pid: None,
        detail,
    })
}
#[cfg(not(coverage))]
/// `launchctl print <domain>/<label>` is the modern state query — a
/// nonzero exit means the job isn't loaded. `installed` tracks launchd's
/// view; `enabled` tracks the plist file (presence = it loads at
/// login/boot).
pub fn status(spec: &ServiceSpec) -> Result<ServiceStatus, CliError> {
    let enabled = unit_path(spec).exists();
    let detail = tail_lines(&log_path(spec), STATUS_LOG_LINES);
    if !on_path("launchctl") {
        return Ok(ServiceStatus {
            installed: false,
            enabled,
            active: false,
            pid: None,
            detail,
        });
    }
    let out = spawn(&["launchctl", "print", &service_target(spec)]);
    if out.code != 0 {
        return Ok(ServiceStatus {
            installed: false,
            enabled,
            active: false,
            pid: None,
            detail,
        });
    }
    let field = |key: &str| {
        out.stdout.lines().find_map(|line| {
            let line = line.trim_start();
            line.strip_prefix(&format!("{key} = ")).map(str::trim)
        })
    };
    let state = field("state");
    let pid = field("pid").and_then(|v| v.parse::<u64>().ok());
    Ok(ServiceStatus {
        installed: true,
        enabled,
        active: state == Some("running") || pid.is_some_and(|p| p > 0),
        pid: pid.filter(|p| *p > 0),
        detail,
    })
}

#[cfg(coverage)]
pub fn restart(_spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()
}
#[cfg(not(coverage))]
/// kickstart -k = kill the running job and start it again on the new plist.
pub fn restart(spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()?;
    run(&["launchctl", "kickstart", "-k", &service_target(spec)])
}

#[cfg(coverage)]
pub fn enable(_spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()
}
#[cfg(not(coverage))]
/// Clear the launchd disabled flag, then load + start the job —
/// bootstrap when it's out of the domain, kickstart when it's loaded
/// but stopped. The plist stays put either way.
pub fn enable(spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()?;
    let _ = spawn(&["launchctl", "enable", &service_target(spec)]);
    if loaded(spec) {
        return run(&["launchctl", "kickstart", "-k", &service_target(spec)]);
    }
    run(&[
        "launchctl",
        "bootstrap",
        &domain(spec),
        &unit_path(spec).to_string_lossy(),
    ])
}

#[cfg(coverage)]
pub fn disable(_spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()
}
#[cfg(not(coverage))]
/// Set the disabled flag (survives login) and boot the job out of the
/// domain now. The plist stays — `enable` brings it back.
pub fn disable(spec: &ServiceSpec) -> Result<(), CliError> {
    require_launchctl()?;
    let _ = spawn(&["launchctl", "disable", &service_target(spec)]);
    if loaded(spec) {
        run(&[
            "launchctl",
            "bootout",
            &domain(spec),
            &unit_path(spec).to_string_lossy(),
        ])?;
    }
    Ok(())
}

#[cfg(coverage)]
pub fn logs(spec: &ServiceSpec, _follow: bool) -> Result<i32, CliError> {
    let _ = log_path(spec);
    Ok(0)
}
#[cfg(not(coverage))]
/// Print the tail; `--follow` hands the terminal to `tail -f`.
pub fn logs(spec: &ServiceSpec, follow: bool) -> Result<i32, CliError> {
    let log = log_path(spec);
    if follow {
        let status = Command::new("tail")
            .args(["-n", &LOG_LINES.to_string(), "-f"])
            .arg(&log)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| CliError(format!("launchd: `tail -f` failed to spawn: {e}")))?;
        return Ok(status.code().unwrap_or(0));
    }
    let tail = tail_lines(&log, LOG_LINES);
    if !tail.is_empty() {
        println!("{tail}");
    }
    Ok(0)
}

#[cfg(test)]
mod cov_tests {
    //! Private-helper coverage the integration tests can't reach —
    //! `launchctl` never exists on this host, so the plumbing runs
    //! against binaries that do (`true`/`false`/`sh`).
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn spec(system: bool) -> ServiceSpec {
        ServiceSpec {
            name: "peregrin".into(),
            exec: vec!["/bin/true".into()],
            env_file: PathBuf::from("/nonexistent-env-file"),
            system,
        }
    }

    #[test]
    fn helpers() {
        let user = spec(false);
        let sys = spec(true);
        let _ = uid();
        let _ = home_dir();
        assert_eq!(label(&user), "ai.peregrin");
        assert_eq!(domain(&sys), "system");
        assert_eq!(domain(&user), format!("gui/{}", uid()));
        assert!(service_target(&user).ends_with("/ai.peregrin"));
        assert!(log_path(&sys).starts_with("/var/log/"));
        assert!(log_path(&user).ends_with("Library/Logs/peregrin.log"));
        assert!(unit_path(&sys).starts_with("/Library/LaunchDaemons/"));
        assert!(unit_path(&user).ends_with("Library/LaunchAgents/ai.peregrin.plist"));
        assert_eq!(xml_escape("<a&b>"), "&lt;a&amp;b&gt;");
        assert!(valid_env_key("A_B1"));
        assert!(!valid_env_key("1A"));
        assert!(!valid_env_key(""));
        assert!(!valid_env_key("A-B"));
    }

    #[test]
    fn env_file_and_log_parsing() {
        // Missing file → empty.
        assert!(parse_env_file(Path::new("/nonexistent-env-file")).is_empty());
        assert!(tail_lines(Path::new("/nonexistent-log"), 5).is_empty());
        let tmp = tempfile::tempdir().unwrap();
        let env = tmp.path().join("env");
        std::fs::write(
            &env,
            "# comment\n\nA=1\nBAD-KEY=x\n=nokey\nNOEQ\nB= 2 \nA=3\n",
        )
        .unwrap();
        // Blank/comment/invalid lines drop; the duplicate collapses to
        // the last value at the first slot's position.
        assert_eq!(
            parse_env_file(&env),
            vec![
                ("A".to_string(), "3".to_string()),
                ("B".to_string(), "2".to_string())
            ]
        );
        let log = tmp.path().join("log");
        std::fs::write(&log, "a\nb\nc\n").unwrap();
        assert_eq!(tail_lines(&log, 2), "b\nc");
        assert_eq!(tail_lines(&log, 9), "a\nb\nc");
    }

    #[test]
    fn render_both_env_arms() {
        // No env file → no EnvironmentVariables dict.
        let bare = render(&spec(false));
        assert!(!bare.contains("EnvironmentVariables"));
        let tmp = tempfile::tempdir().unwrap();
        let env = tmp.path().join("env");
        std::fs::write(&env, "K=V&<>\n").unwrap();
        let mut with_env = spec(false);
        with_env.env_file = env;
        let plist = render(&with_env);
        assert!(plist.contains("EnvironmentVariables"));
        assert!(plist.contains("<key>K</key>"));
        assert!(plist.contains("V&amp;&lt;&gt;"));
    }

    #[test]
    fn launchctl_plumbing() {
        assert_eq!(spawn(&["true"]).code, 0);
        assert_ne!(spawn(&["false"]).code, 0);
        let missing = spawn(&["definitely-not-on-path-xyz"]);
        assert_eq!(missing.code, 1);
        assert!(!missing.stderr.is_empty());
        let empty = spawn(&[]);
        assert_eq!(empty.code, 1);
        assert_eq!(empty.stderr, "empty command");
        assert!(run(&["true"]).is_ok());
        assert!(run(&["false"]).is_err()); // empty detail arm
        assert!(run(&["sh", "-c", "echo out; exit 3"]).is_err()); // stdout detail
        assert!(run(&["sh", "-c", "echo e >&2; exit 3"]).is_err()); // stderr detail
        assert!(!loaded(&spec(false)));
        assert!(require_bin("sh").is_ok());
        assert!(require_bin("definitely-not-on-path-xyz").is_err());
        assert!(require_launchctl().is_err());
    }

    #[test]
    fn uninstall_paths() {
        // A system spec's unit path can't be removed without root — the
        // best-effort delete no-ops; `purge` still drops the env file.
        let tmp = tempfile::tempdir().unwrap();
        let env = tmp.path().join("env");
        std::fs::write(&env, "K=V").unwrap();
        let mut s = spec(true);
        s.env_file = env;
        assert!(uninstall(&s, true).is_ok());
        assert!(!s.env_file.exists());
        assert!(uninstall(&s, false).is_ok());
    }
}
