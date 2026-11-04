//! TOML impl of `ConfigSource` — reads one file as TOML and
//! normalizes it into the workflow-shaped node config
//! ([`peregrin_core::Campaign`], ADR-0080). Path resolution/search
//! lives in `peregrin-config-file`'s dispatcher; this adapter only
//! owns the TOML syntax. Parsed campaigns get `~` expanded in their
//! local-path fields (`peregrin_config::expand_campaign_paths`), so
//! TOML and YAML configs carry identical semantics.

use peregrin_config::{ConfigSource, campaign_from_raw};
use std::path::PathBuf;

/// TOML config file — `path` is the file to read (already resolved).
pub struct TomlSource {
    /// Config file path.
    pub path: PathBuf,
}

impl ConfigSource for TomlSource {
    fn load(&self) -> Result<peregrin_core::Campaign, String> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("read {}: {e}", self.path.display()))?;
        let raw: serde_json::Value =
            toml::from_str(&text).map_err(|e| format!("parse {}: {e}", self.path.display()))?;
        campaign_from_raw(&raw, &self.path)
    }
}
