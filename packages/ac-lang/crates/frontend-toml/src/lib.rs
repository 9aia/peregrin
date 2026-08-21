//! # ac-frontend-toml
//!
//! TOML frontend for the AC language.
//!
//! Translates TOML into the core [`NonFunctionValue`] representation.
//! TOML is a frontend — it produces the same value type the VM
//! operates on. No execution logic lives here.
//!
//! ## Architectural rule
//!
//! > **Formats translate external representations.**

mod convert;
mod error;
mod scanner;

pub use convert::{json_to_toml, toml_to_json};
pub use error::TomlError;

use ac_runtime::{Frontend, NonFunctionValue};

/// Parses a TOML string into a [`NonFunctionValue`].
///
/// TOML is a frontend — it produces the same core [`NonFunctionValue`]
/// representation that the VM operates on. No execution logic
/// lives here.
///
/// # Example
///
/// ```ignore
/// use ac_frontend_toml::parse;
///
/// let value = parse("name = \"ac\"\nversion = 1\n").unwrap();
/// assert_eq!(value.get("name").unwrap().as_str(), Some("ac"));
/// ```
pub fn parse(input: &str) -> Result<NonFunctionValue, TomlError> {
    let parsed: toml::Value = toml::from_str(input)?;
    Ok(from_toml(parsed))
}

/// Converts a `toml::Value` into a core [`NonFunctionValue`].
pub fn from_toml(v: toml::Value) -> NonFunctionValue {
    NonFunctionValue::new(toml_to_json(v))
}

/// Serializes a [`NonFunctionValue`] to a TOML string.
///
/// TOML requires a top-level table (object). If the value is not
/// an object, it is wrapped in a single-key table under `value`.
pub fn to_string(value: &NonFunctionValue) -> Result<String, TomlError> {
    let toml_val = json_to_toml(value.inner());
    match toml_val {
        toml::Value::Table(_) => Ok(toml::to_string(&toml_val)?),
        _ => {
            let mut table = toml::Table::new();
            table.insert("value".to_string(), toml_val);
            Ok(toml::to_string(&table)?)
        }
    }
}

/// Converts a core [`NonFunctionValue`] into a `toml::Value`.
pub fn to_toml(value: &NonFunctionValue) -> toml::Value {
    json_to_toml(value.inner())
}

/// TOML frontend implementing the [`Frontend`] trait.
#[derive(Debug, Clone, Copy, Default)]
pub struct TomlFrontend;

impl Frontend for TomlFrontend {
    type Error = TomlError;

    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        parse(input)
    }

    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        to_string(value)
    }

    fn parse_with_locations(
        &self,
        input: &str,
    ) -> Result<(NonFunctionValue, ac_source::SourceMap), Self::Error> {
        let value = parse(input)?;
        let map = scanner::build_source_map(input, value.inner());
        Ok((value, map))
    }
}

#[cfg(test)]
mod convert_tests;
