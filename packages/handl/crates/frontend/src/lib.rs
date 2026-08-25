//! # handl-frontend
//!
//! Frontend factory — maps format names to `Frontend` implementations.
//!
//! Provides a single entry point [`frontend_for`] that returns a boxed
//! `Frontend` for the requested format. Each frontend wraps the raw
//! parse/serialize functions from the format-specific crates.
//!
//! This crate is interface-agnostic — usable from CLI, MCP, or any
//! other consumer that needs to parse/serialize handl documents.

mod handl;
mod json;
mod toml;
mod yaml;

use handl_vm_values_json::Frontend;

pub use handl::HandlFrontend;
pub use json::JsonFrontend;
pub use toml::TomlFrontend;
pub use yaml::YamlFrontend;

/// Return a `Frontend` for the given format name.
///
/// Supported formats: `yaml`/`yml`, `json`, `toml`, `handl`.
pub fn frontend_for(format: &str) -> Result<Box<dyn Frontend<Error = String>>, String> {
    match format {
        "yaml" | "yml" => Ok(Box::new(YamlFrontend)),
        "json" => Ok(Box::new(JsonFrontend)),
        "toml" => Ok(Box::new(TomlFrontend)),
        "handl" => Ok(Box::new(HandlFrontend)),
        other => Err(format!("unknown format: {other}")),
    }
}
