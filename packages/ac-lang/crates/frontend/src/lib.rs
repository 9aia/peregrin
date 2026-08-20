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

use ac_runtime::{Frontend, RuntimeValue};
use ac_source::SourceMap;

/// Return a `Frontend` for the given format name.
///
/// Supported formats: `yaml`/`yml`, `json`, `toml`.
pub fn frontend_for(format: &str) -> Result<Box<dyn Frontend<Error = String>>, String> {
    match format {
        "yaml" | "yml" => Ok(Box::new(YamlFrontend)),
        "json" => Ok(Box::new(JsonFrontend)),
        "toml" => Ok(Box::new(TomlFrontend)),
        other => Err(format!("unknown format: {other}")),
    }
}

pub struct YamlFrontend;
impl Frontend for YamlFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<RuntimeValue, Self::Error> {
        ac_frontend_yaml::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &RuntimeValue) -> Result<String, Self::Error> {
        ac_frontend_yaml::to_string(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(&self, input: &str) -> Result<(RuntimeValue, SourceMap), Self::Error> {
        ac_frontend_yaml::YamlFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}

pub struct JsonFrontend;
impl Frontend for JsonFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<RuntimeValue, Self::Error> {
        ac_frontend_json::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &RuntimeValue) -> Result<String, Self::Error> {
        ac_frontend_json::to_string_pretty(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(&self, input: &str) -> Result<(RuntimeValue, SourceMap), Self::Error> {
        ac_frontend_json::JsonFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}

pub struct TomlFrontend;
impl Frontend for TomlFrontend {
    type Error = String;
    fn parse(&self, input: &str) -> Result<RuntimeValue, Self::Error> {
        ac_frontend_toml::parse(input).map_err(|e| e.to_string())
    }
    fn serialize(&self, value: &RuntimeValue) -> Result<String, Self::Error> {
        ac_frontend_toml::to_string(value).map_err(|e| e.to_string())
    }
    fn parse_with_locations(&self, input: &str) -> Result<(RuntimeValue, SourceMap), Self::Error> {
        ac_frontend_toml::TomlFrontend
            .parse_with_locations(input)
            .map_err(|e| e.to_string())
    }
}
