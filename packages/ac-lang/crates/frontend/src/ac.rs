//! # AC text frontend
//!
//! Passes through raw canonical AC source as a string. The string is
//! intended to be lexed and parsed by `ac-lexer-canonical` / `ac-parser-canonical`
//! downstream (e.g. in the `EvalStage`), rather than being interpreted as a
//! JSON-compatible document.

use ac_runtime_values::NonFunctionValue;
use ac_source::SourceMap;

/// Frontend for the canonical AC expression text format.
pub struct AcFrontend;

impl ac_runtime_values::Frontend for AcFrontend {
    type Error = String;

    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error> {
        Ok(NonFunctionValue::new(input.to_string()))
    }

    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error> {
        match value.as_str() {
            Some(s) => Ok(s.to_string()),
            None => Err("AC frontend can only serialize string values".to_string()),
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
