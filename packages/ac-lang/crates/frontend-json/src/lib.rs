//! # ac-frontend-json
//!
//! JSON frontend for the AC language.
//!
//! Translates JSON into the core [`NonFunctionValue`] representation.
//! Since `NonFunctionValue` wraps `serde_json::Value`, this frontend is
//! a thin adapter — parsing produces a `NonFunctionValue` directly.
//!
//! ## Architectural rule
//!
//! > **Formats translate external representations.**

mod scanner;

use ac_runtime::{Frontend, NonFunctionValue};
use thiserror::Error;

/// Errors that can occur during JSON parsing.
#[derive(Debug, Error)]
pub enum JsonError {
    /// The underlying JSON parser returned an error.
    #[error("json parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Parses a JSON string into a [`NonFunctionValue`].
///
/// JSON is a frontend — it produces the same core [`NonFunctionValue`]
/// representation that the VM operates on. No execution logic
/// lives here.
///
/// # Example
///
/// ```ignore
/// use ac_frontend_json::parse;
///
/// let value = parse(r#"{"name": "ac", "version": 1}"#).unwrap();
/// assert_eq!(value.get("name").unwrap().as_str(), Some("ac"));
/// ```
pub fn parse(input: &str) -> Result<NonFunctionValue, JsonError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    Ok(NonFunctionValue::new(parsed))
}

/// Converts a `serde_json::Value` into a core [`NonFunctionValue`].
pub fn from_json(v: serde_json::Value) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

/// Serializes a [`NonFunctionValue`] to a JSON string.
pub fn to_string(value: &NonFunctionValue) -> Result<String, JsonError> {
    Ok(serde_json::to_string(value.inner())?)
}

/// Serializes a [`NonFunctionValue`] to a pretty-printed JSON string.
pub fn to_string_pretty(value: &NonFunctionValue) -> Result<String, JsonError> {
    Ok(serde_json::to_string_pretty(value.inner())?)
}

/// Converts a core [`NonFunctionValue`] into a `serde_json::Value`.
pub fn to_json(value: &NonFunctionValue) -> serde_json::Value {
    value.inner().clone()
}

/// JSON frontend implementing the [`Frontend`] trait.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonFrontend;

impl Frontend for JsonFrontend {
    type Error = JsonError;

    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        parse(input)
    }

    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        to_string_pretty(value)
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
mod lib_tests;
