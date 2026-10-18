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
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, lines.join("\n") + "\n")
    }
}
