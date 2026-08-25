//! # handl-lib-toml
//!
//! TOML `Encoder`/`Decoder` for the `handl-lib-serialize` framework.
//!
//! Backed by `toml`.

use handl_lib_serialize::{Decoder, Encoder, SerializeError};
use toml::Value;

/// Combined TOML codec.
#[derive(Debug, Clone, Copy, Default)]
pub struct Toml;

/// TOML encoder.
#[derive(Debug, Clone, Copy, Default)]
pub struct TomlEncoder;

/// TOML decoder.
#[derive(Debug, Clone, Copy, Default)]
pub struct TomlDecoder;

impl Encoder<Value> for Toml {
    type Output = String;

    fn encode(&self, value: &Value) -> Result<Self::Output, SerializeError> {
        toml::to_string(value).map_err(|e| SerializeError::new(e.to_string()))
    }
}

impl Decoder<str, Value> for Toml {
    fn decode(&self, input: &str) -> Result<Value, SerializeError> {
        toml::from_str(input).map_err(|e| SerializeError::new(e.to_string()))
    }
}

impl Encoder<Value> for TomlEncoder {
    type Output = String;

    fn encode(&self, value: &Value) -> Result<Self::Output, SerializeError> {
        Toml.encode(value)
    }
}

impl Decoder<str, Value> for TomlDecoder {
    fn decode(&self, input: &str) -> Result<Value, SerializeError> {
        Toml.decode(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_document() {
        let input = r#"
title = "example"
[owner]
name = "luis"
nums = [1, 2, 3]
active = true
"#;
        let value = Toml.decode(input).unwrap();
        let output = Toml.encode(&value).unwrap();
        let round = Toml.decode(&output).unwrap();
        assert_eq!(value, round);
    }

    #[test]
    fn rejects_invalid_toml() {
        assert!(Toml.decode("not toml").is_err());
    }
}
