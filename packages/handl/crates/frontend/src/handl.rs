//! # handl text frontend
//!
//! Passes through raw canonical handl source as a string. The string is
//! intended to be lexed and parsed by `handl-lexer-canonical` / `handl-parser-canonical`
//! downstream (e.g. in the `EvalStage`), rather than being interpreted as a
//! JSON-compatible document.

use handl_source::SourceMap;
use handl_vm_values_json::NonFunctionValue;

/// Frontend for the canonical handl expression text format.
pub struct HandlFrontend;

impl handl_vm_values_json::Frontend for HandlFrontend {
    type Error = String;

    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        Ok(NonFunctionValue::new(input.to_string()))
    }

    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        match value.as_str() {
            Some(s) => Ok(s.to_string()),
            None => Err("handl frontend can only serialize string values".to_string()),
        }
    }

    fn parse_with_locations(
        &self,
        input: &str,
    ) -> Result<(NonFunctionValue, SourceMap), Self::Error> {
        let value = self.parse(input)?;
        Ok((value, SourceMap::new()))
    }
}
