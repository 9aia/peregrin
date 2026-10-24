//! peregrin-dotenv — `KEY=VAL` file impl of `Secrets` (sturn pattern).

use std::path::PathBuf;

/// A `.env`-style file — the always-available fallback in the secret
/// chain.
pub struct DotenvSecrets {
    pub path: PathBuf,
}

impl DotenvSecrets {
    fn read(&self) -> String {
        std::fs::read_to_string(&self.path).unwrap_or_default()
    }
}

impl peregrin_secret::Secrets for DotenvSecrets {
    fn get(&self, name: &str) -> Option<String> {
        self.read()
            .lines()
            .find_map(|l| l.strip_prefix(name)?.strip_prefix('=').map(str::to_string))
            .filter(|v| !v.is_empty())
    }

    fn set(&self, name: &str, value: &str) -> std::io::Result<()> {
        let mut lines: Vec<String> = self
            .read()
            .lines()
            .filter(|l| !l.starts_with(&format!("{name}=")))
            .map(str::to_string)
            .collect();
        lines.push(format!("{name}={value}"));
        // `parent()` is never `None` for a real path — `map_or` keeps the
        // call unconditional rather than leaving an else-arm region.
        self.path.parent().map_or(Ok(()), std::fs::create_dir_all)?;
        std::fs::write(&self.path, lines.join("\n") + "\n")
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
