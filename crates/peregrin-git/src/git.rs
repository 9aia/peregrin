//! The git seam — everything [`Worktrees`](crate::Worktrees) asks of
//! git is a `run(dir, args)`, so callers can swap [`SystemGit`] for a
//! fake or a hermetic toolchain.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::Error;

/// A git invocation: run `git <args>` with `dir` as the working
/// directory, returning stdout on success.
pub trait Git: Send + Sync {
    /// `Err` on spawn failure or a non-zero exit — [`Error::Git`]
    /// carries stderr where git produced one.
    fn run(&self, dir: &Path, args: &[&OsStr]) -> Result<String, Error>;
}

/// [`Git`] over the `git` binary. The child sees the sanitized
/// [`peregrin_core::child_env`] allowlist — the caller's whole
/// environment routinely carries secrets a subprocess must not
/// inherit.
#[derive(Clone)]
pub struct SystemGit {
    program: OsString,
}

impl Default for SystemGit {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGit {
    /// `git` resolved through `PATH`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: OsString::from("git"),
        }
    }

    /// A different binary or path (fakes, pinned toolchains).
    #[must_use]
    pub fn with_program(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
        }
    }
}

impl Git for SystemGit {
    fn run(&self, dir: &Path, args: &[&OsStr]) -> Result<String, Error> {
        let output = Command::new(&self.program)
            .args(args)
            .current_dir(dir)
            .env_clear()
            .envs(peregrin_core::child_env())
            .stdin(Stdio::null())
            .output()
            .map_err(|source| Error::Spawn {
                program: self.program.to_string_lossy().into_owned(),
                source,
            })?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(Error::Git {
                dir: dir.to_path_buf(),
                args: args
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            })
        }
    }
}
