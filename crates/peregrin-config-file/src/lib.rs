//! File impl of `ConfigSource` — resolves which config file to load:
//! explicit path → `./peregrin.*` → `~/.config/peregrin/peregrin.*`;
//! at each search step YAML wins when several formats exist. Syntax
//! is dispatched by extension to the per-syntax adapters:
//! `.yaml`/`.yml` → `peregrin-config-yaml`, everything else →
//! `peregrin-config-toml` (back-compat for explicit/extensionless
//! paths).
//!
//! No `$PEREGRIN_CONFIG` env var or `--config` flag exists in the
//! codebase yet — [`FileSource::path`] is the seam a flag will fill.

use peregrin_config::ConfigSource;
use peregrin_config_toml::TomlSource;
use peregrin_config_yaml::YamlSource;
use std::path::{Path, PathBuf};

/// File config source — resolves + loads the node's
/// [`peregrin_core::Campaign`]-shaped config document.
pub struct FileSource {
    /// Explicit `--config`-style path, or None for the default search.
    pub path: Option<String>,
}

/// File names tried, in order, inside each searched directory —
/// YAML first (preferred), TOML last (permanent fallback).
const NAMES: [&str; 3] = ["peregrin.yaml", "peregrin.yml", "peregrin.toml"];

/// Which syntax a config path carries — `.yaml`/`.yml` parse as
/// YAML, everything else as TOML (back-compat for explicit paths).
fn is_yaml(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()), Some("yaml" | "yml"))
}

impl ConfigSource for FileSource {
    fn load(&self) -> Result<peregrin_core::Campaign, String> {
        let candidates: Vec<PathBuf> = if let Some(p) = self.path.clone() {
            vec![PathBuf::from(p)]
        } else {
            // `expand_tilde` carries the HOME/USERPROFILE/`/`
            // home convention already.
            let config_dir = PathBuf::from(peregrin_config::expand_tilde("~/.config/peregrin"));
            let mut v = Vec::with_capacity(NAMES.len() * 2);
            for n in NAMES {
                v.push(PathBuf::from(n));
            }
            for n in NAMES {
                v.push(config_dir.join(n));
            }
            v
        };
        for p in candidates {
            if p.exists() {
                return if is_yaml(&p) {
                    YamlSource { path: p }.load()
                } else {
                    TomlSource { path: p }.load()
                };
            }
        }
        Err(
            "no peregrin.{yaml,yml,toml} found (search: explicit path, ./, ~/.config/peregrin/)"
                .into(),
        )
    }
}
