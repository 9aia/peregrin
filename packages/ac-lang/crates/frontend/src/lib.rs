//! # ac-frontend
//!
//! Frontend factory — maps format names to `Frontend` implementations.
//!
//! Provides a single entry point [`frontend_for`] that returns a boxed
//! `Frontend` for the requested format. Each frontend wraps the raw
//! parse/serialize functions from the format-specific crates.
//!
//! This crate is interface-agnostic — usable from CLI, MCP, or any
//! other consumer that needs to parse/serialize AC documents.

mod json;
mod toml;
mod yaml;

use ac_runtime::Frontend;

pub use json::JsonFrontend;
pub use toml::TomlFrontend;
pub use yaml::YamlFrontend;

/// Return a `Frontend` for the given format name.
///
/// Supported formats: `yaml`/`yml`, `json`, `toml`.
pub fn frontend_for(format: &str) -> Result<Box<dyn Frontend<Error = String>>, String> {
    match format {
        "yaml" | "yml" => Ok(Box::new(YamlFrontend)),
        "json" => Ok(Box::new(JsonFrontend)),
        "toml" => Ok(Box::new(TomlFrontend)),
        other => Err(format!("unknown format: {other}"))}
}
