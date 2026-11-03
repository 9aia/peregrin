//! Ambient effects — the seam behind the I/O and entropy builtins.
//! The TS engine called `process.env`, `fs`, `child_process.exec`,
//! `Date`, `Math.random`, and `crypto.randomUUID` directly; peregrin
//! keeps the engine pure by injecting them, with [`SystemEffects`] as
//! the production std impl.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Effects the builtins need. Errors are strings because the builtins
/// wrap them in `TemplateError { reason: "file() failed: {e}" }` /
/// `"shell() failed: {e}"`.
pub trait Effects: Send + Sync {
    /// `process.env[name]` — `None` when unset.
    fn env(&self, name: &str) -> Option<String>;
    /// `process.cwd()` — base for relative `file()` paths.
    fn cwd(&self) -> PathBuf;
    /// Read raw bytes; the caller decodes utf-8 lossily (`TextDecoder`
    /// with `fatal: false`).
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String>;
    /// Run a command under the system shell; `Ok` is stdout with
    /// trailing whitespace trimmed (`stdout.replace(/\s+$/, "")`).
    fn shell(&self, command: &str) -> Result<String, String>;
    /// `Math.floor(Date.now() / 1000)` — unix epoch seconds.
    fn now_unix(&self) -> i64;
    /// `new Date().toISOString()` — `YYYY-MM-DDTHH:mm:ss.sssZ`.
    fn now_iso(&self) -> String;
    /// `Math.random()` — uniform in `[0, 1)`.
    fn random(&self) -> f64;
    /// `crypto.randomUUID()` — lowercase hyphenated v4.
    fn uuid(&self) -> String;
}

/// Production effects over `std` — env, cwd, fs, `sh -c` (or `cmd /C`),
/// wall clock, and uuid-seeded randomness.
pub struct SystemEffects;

impl Effects for SystemEffects {
    fn env(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn cwd(&self) -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }

    fn shell(&self, command: &str) -> Result<String, String> {
        run_shell(command)
    }

    fn now_unix(&self) -> i64 {
        unix_millis() / 1000
    }

    fn now_iso(&self) -> String {
        iso_utc(unix_millis())
    }

    fn random(&self) -> f64 {
        // Same 53-bit mantissa draw as `Math.random`: take 53 high bits
        // of a random u64 into [0, 2^53), scale into [0, 1).
        let bytes = uuid::Uuid::new_v4().into_bytes();
        let bits = u64::from_be_bytes(bytes[..8].try_into().unwrap_or_default());
        #[allow(clippy::cast_precision_loss)]
        ((bits >> 11) as f64 / 9_007_199_254_740_992.0)
    }

    fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// Node `exec` uses `/bin/sh -c` on unix, `cmd.exe` on Windows. The
/// child gets the sanitized env (`peregrin_core::child_env`) — a
/// `${shell()}` inside a resolved param must not inherit the daemon's
/// ambient secrets.
#[cfg(unix)]
fn run_shell(command: &str) -> Result<String, String> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    shell_output(&mut cmd)
}

/// Windows equivalent (`cmd /C`).
#[cfg(windows)]
fn run_shell(command: &str) -> Result<String, String> {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command]);
    shell_output(&mut cmd)
}

fn shell_output(cmd: &mut Command) -> Result<String, String> {
    cmd.env_clear().envs(peregrin_core::child_env());
    match cmd.output() {
        Err(e) => Err(e.to_string()),
        Ok(out) if !out.status.success() => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stderr = stderr.trim_end();
            if stderr.is_empty() {
                Err(format!("Command failed: {}", out.status))
            } else {
                Err(format!("Command failed: {stderr}"))
            }
        }
        Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string()),
    }
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// `new Date().toISOString()` — `YYYY-MM-DDTHH:mm:ss.sssZ` from epoch ms.
fn iso_utc(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let hour = rem / 3_600_000;
    let minute = rem % 3_600_000 / 60_000;
    let second = rem % 60_000 / 1000;
    let milli = rem % 1000;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

/// Days-from-epoch → (year, month, day); Howard Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn iso_shape() {
        // 1970-01-01T00:00:00.000Z and a known instant.
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_utc(1_700_000_000_123), "2023-11-14T22:13:20.123Z");
        // Leap day.
        assert_eq!(iso_utc(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[cfg(unix)]
    #[test]
    fn shell_does_not_inherit_ambient_env() {
        // Re-exec with a marker var — edition-2024 `set_var` is
        // unsafe, so the child process env is the clean seam.
        if std::env::var_os("PERE_TEST_SCRUB").is_none() {
            let exe = std::env::current_exe().unwrap();
            let out = std::process::Command::new(exe)
                .args([
                    "effects::tests::shell_does_not_inherit_ambient_env",
                    "--exact",
                    "--nocapture",
                ])
                .env("PERE_TEST_SCRUB", "zzz-secret")
                .output()
                .unwrap();
            assert!(out.status.success());
            return;
        }
        let stdout = SystemEffects
            .shell("printf %s \"[$PERE_TEST_SCRUB]\"")
            .unwrap();
        assert_eq!(stdout, "[]");
    }
}
