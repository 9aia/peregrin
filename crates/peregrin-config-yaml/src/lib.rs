//! YAML impl of `ConfigSource` — reads one file as YAML (`serde_norway`
//! transcode: `Value` → `serde_json::Value` → `Campaign::from_dict`,
//! the same normalization funnel the TOML adapter takes) into the
//! workflow-shaped node config ([`peregrin_core::Campaign`], ADR-0080).
//! Path resolution/search lives in `peregrin-config-file`'s
//! dispatcher; this adapter only owns the YAML syntax. Parsed
//! campaigns get `~` expanded in their local-path fields
//! (`peregrin_config::expand_campaign_paths`), so YAML and TOML
//! configs carry identical semantics.

use peregrin_config::{ConfigSource, campaign_from_raw};
use std::path::PathBuf;

/// YAML config file — `path` is the file to read (already resolved).
pub struct YamlSource {
    /// Config file path.
    pub path: PathBuf,
}

impl ConfigSource for YamlSource {
    fn load(&self) -> Result<peregrin_core::Campaign, String> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("read {}: {e}", self.path.display()))?;
        let value: serde_norway::Value = serde_norway::from_str(&text)
            .map_err(|e| format!("parse {}: {e}", self.path.display()))?;
        let raw: serde_json::Value = serde_norway::from_value(value)
            .map_err(|e| format!("parse {}: {e}", self.path.display()))?;
        campaign_from_raw(&raw, &self.path)
    }
}
