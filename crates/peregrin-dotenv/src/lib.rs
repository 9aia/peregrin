//! peregrin-dotenv — `KEY=VAL` file impl of `Secrets` (sturn pattern).

use std::path::PathBuf;

/// A `.env`-style file — the always-available fallback in the secret
/// chain.
pub struct DotenvSecrets {
    pub path: PathBuf,
}

/// A usable entry name — non-empty, no `=`/newlines (either would
/// corrupt the file or inject extra entries on write).
fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('=') && !name.contains(['\n', '\r'])
}

/// A usable value — no newlines; `.env` is one entry per line, so a
/// multi-line value would smuggle in extra entries.
fn valid_value(value: &str) -> bool {
    !value.contains(['\n', '\r'])
}

fn invalid_input(detail: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, detail)
}

/// Write `contents` to `path` atomically (tmp + rename) at 0600 on
/// unix — the file holds secrets; umask defaults would leave it
/// group/world-readable.
fn write_secret_file(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    path.parent().map_or(Ok(()), std::fs::create_dir_all)?;
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path)
}

impl DotenvSecrets {
    fn read(&self) -> String {
        std::fs::read_to_string(&self.path).unwrap_or_default()
    }
}

impl peregrin_secret::Secrets for DotenvSecrets {
    fn get(&self, name: &str) -> Option<String> {
        if !valid_name(name) {
            return None;
        }
        self.read()
            .lines()
            .find_map(|l| l.strip_prefix(name)?.strip_prefix('=').map(str::to_string))
            .filter(|v| !v.is_empty())
    }

    fn set(&self, name: &str, value: &str) -> std::io::Result<()> {
        if !valid_name(name) {
            return Err(invalid_input(
                "secret name must be non-empty without '=' or newlines",
            ));
        }
        if !valid_value(value) {
            return Err(invalid_input("secret value must not contain newlines"));
        }
        let mut lines: Vec<String> = self
            .read()
            .lines()
            .filter(|l| !l.starts_with(&format!("{name}=")))
            .map(str::to_string)
            .collect();
        lines.push(format!("{name}={value}"));
        write_secret_file(&self.path, &format!("{}\n", lines.join("\n")))
    }
}

#[cfg(test)]
mod cov_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use peregrin_secret::Secrets as _;

    #[test]
    fn set_on_an_unwritable_parent_errors() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        std::fs::write(&blocker, "x").unwrap();
        let env = DotenvSecrets {
            path: blocker.join("nested.env"),
        };
        assert!(env.set("A", "1").is_err());
    }
}
