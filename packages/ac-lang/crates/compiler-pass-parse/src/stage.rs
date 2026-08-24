//! driver-lang Stage implementation for the parse phase.

use ac_compiler::driver::{DriverError, ParsedDoc, Session, Stage};
use ac_compiler::{Config, Source};

/// Parse phase: `Source → ParsedDoc`.
///
/// Parses source text into a `NonFunctionValue` using the format-specified
/// frontend. Builds a source map for later error location resolution.
pub struct ParseStage;

impl Stage<Config> for ParseStage {
    type Input = Source;
    type Output = ParsedDoc;

    fn name(&self) -> &'static str {
        "parse"
    }

    fn run(
        &mut self,
        source: Source,
        _session: &mut Session<Config>,
    ) -> Result<ParsedDoc, DriverError> {
        let base_path = source.base_path();
        let frontend =
            ac_frontend::frontend_for(&source.format).map_err(|e| DriverError::new(e))?;
        let (value, source_map) = frontend
            .parse_with_locations(&source.input)
            .map_err(|e| DriverError::new(e))?;
        Ok(ParsedDoc {
            value,
            source_map,
            base_path,
            format: source.format.to_string(),
        })
    }
}