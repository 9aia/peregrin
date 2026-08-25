//! # handl-frontend-yaml
//!
//! YAML frontend for the handl language.
//!
//! Translates YAML into the core [`NonFunctionValue`] representation.
//! YAML is a frontend — it produces the same value type the VM
//! operates on. No execution logic lives here.
//!
//! ## Architectural rule
//!
//! > **Formats translate external representations.**

mod convert;
mod error;
mod scanner;

pub use convert::{json_to_yaml, yaml_to_json};
pub use error::YamlError;

use handl_vm_values_json::{Frontend, NonFunctionValue};
use serde_json::Value as JsonValue;

/// Parses a YAML string into a [`NonFunctionValue`].
///
/// YAML is a frontend — it produces the same core [`NonFunctionValue`]
/// representation that the VM operates on. No execution logic
/// lives here.
///
/// # Example
///
/// ```ignore
/// use handl_frontend_yaml::parse;
///
/// let value = parse("name: handl\nversion: 1\n").unwrap();
/// assert_eq!(value.get("name").unwrap().as_str(), Some("handl"));
/// ```
pub fn parse(input: &str) -> Result<NonFunctionValue, YamlError> {
    let parsed: serde_yaml::Value = serde_yaml::from_str(input)?;
    let json = yaml_to_json(parsed)?;
    Ok(NonFunctionValue::new(json))
}

/// Converts a `serde_yaml::Value` into a core [`NonFunctionValue`].
pub fn from_yaml(v: serde_yaml::Value) -> NonFunctionValue {
    let json = yaml_to_json(v).unwrap_or(JsonValue::Null);
    NonFunctionValue::new(json)
}

/// Serializes a [`NonFunctionValue`] to a YAML string.
pub fn to_string(value: &NonFunctionValue) -> Result<String, YamlError> {
    let yaml = json_to_yaml(value.inner());
    Ok(serde_yaml::to_string(&yaml)?)
}

/// Converts a core [`NonFunctionValue`] into a `serde_yaml::Value`.
pub fn to_yaml(value: &NonFunctionValue) -> serde_yaml::Value {
    json_to_yaml(value.inner())
}

/// YAML frontend implementing the [`Frontend`] trait.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlFrontend;

impl Frontend for YamlFrontend {
    type Error = YamlError;

    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        parse(input)
    }

    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        to_string(value)
    }

    fn parse_with_locations(
        &self,
        input: &str,
    ) -> Result<(NonFunctionValue, handl_source::SourceMap), Self::Error> {
        let value = parse(input)?;
        let map = scanner::build_source_map(input, value.inner());
        Ok((value, map))
    }
}

#[cfg(test)]
mod convert_tests;
