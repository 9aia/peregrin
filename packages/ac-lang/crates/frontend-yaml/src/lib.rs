//! # ac-frontend-yaml
//!
//! YAML frontend for the AC language.
//!
//! Translates YAML into the core [`RuntimeValue`] representation.
//! YAML is a frontend — it produces the same value type the VM
//! operates on. No execution logic lives here.
//!
//! ## Architectural rule
//!
//! > **Formats translate external representations.**

mod error;
mod convert;
mod scanner;

pub use error::YamlError;
pub use convert::{yaml_to_json, json_to_yaml};

use ac_runtime::{Frontend, RuntimeValue};
use serde_json::Value as JsonValue;

/// Parses a YAML string into a [`RuntimeValue`].
///
/// YAML is a frontend — it produces the same core [`RuntimeValue`]
/// representation that the VM operates on. No execution logic
/// lives here.
///
/// # Example
///
/// ```ignore
/// use ac_frontend_yaml::parse;
///
/// let value = parse("name: ac\nversion: 1\n").unwrap();
/// assert_eq!(value.get("name").unwrap().as_str(), Some("ac"));
/// ```
pub fn parse(input: &str) -> Result<RuntimeValue, YamlError> {
    let parsed: serde_yaml::Value = serde_yaml::from_str(input)?;
    let json = yaml_to_json(parsed)?;
    Ok(RuntimeValue::new(json))
}

/// Converts a `serde_yaml::Value` into a core [`RuntimeValue`].
pub fn from_yaml(v: serde_yaml::Value) -> RuntimeValue {
    let json = yaml_to_json(v).unwrap_or(JsonValue::Null);
    RuntimeValue::new(json)
}

/// Serializes a [`RuntimeValue`] to a YAML string.
pub fn to_string(value: &RuntimeValue) -> Result<String, YamlError> {
    let yaml = json_to_yaml(value.inner());
    Ok(serde_yaml::to_string(&yaml)?)
}

/// Converts a core [`RuntimeValue`] into a `serde_yaml::Value`.
pub fn to_yaml(value: &RuntimeValue) -> serde_yaml::Value {
    json_to_yaml(value.inner())
}

/// YAML frontend implementing the [`Frontend`] trait.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlFrontend;

impl Frontend for YamlFrontend {
    type Error = YamlError;

    fn parse(&self, input: &str) -> Result<RuntimeValue, Self::Error> {
        parse(input)
    }

    fn serialize(&self, value: &RuntimeValue) -> Result<String, Self::Error> {
        to_string(value)
    }

    fn parse_with_locations(&self, input: &str) -> Result<(RuntimeValue, ac_source::SourceMap), Self::Error> {
        let value = parse(input)?;
        let map = scanner::build_source_map(input, value.inner());
        Ok((value, map))
    }
}

#[cfg(test)]
mod convert_tests;
