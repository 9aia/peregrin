//! TOML impl of `ConfigSource` — reads one file as TOML and
//! normalizes it into the workflow-shaped node config
//! ([`peregrin_core::Campaign`], ADR-0080). Path resolution/search
//! lives in scarab-config's file discovery (P-017); this adapter
//! only owns the TOML syntax — parsing rides `scarab-config-toml`'s
//! `TomlFormat` over `serde_json::Value`, the same document shape
//! `toml::from_str` produced. Parsed campaigns get `~` expanded in
//! their local-path fields (`peregrin_config::expand_campaign_paths`),
//! so TOML and YAML configs carry identical semantics.

use peregrin_config::{ConfigSource, campaign_from_raw};
use scarab_config::ConfigFormat;
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
        // scarab's TomlFormat parses the document into
        // `serde_json::Value`; its "parse toml config: " prefix is
        // folded back into the historical "parse <path>: <err>" shape.
        let raw: serde_json::Value =
            ConfigFormat::<serde_json::Value>::parse(&scarab_config_toml::TomlFormat, &text)
                .map_err(|e| {
                    let detail = e.strip_prefix("parse toml config: ").unwrap_or(e.as_str());
                    format!("parse {}: {detail}", self.path.display())
                })?;
        campaign_from_raw(&raw, &self.path)
    }
}
