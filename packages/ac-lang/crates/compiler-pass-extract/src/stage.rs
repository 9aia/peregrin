//! driver-lang Stage implementation for the extract phase.

use ac_compiler::Config;
use ac_compiler::driver::{DriverError, ExtractedDoc, ParsedDoc, Session, Stage};

/// Extract phase: `ParsedDoc → ExtractedDoc`.
///
/// Extracts fields, functions, and data from the parsed document.
pub struct ExtractStage;

impl Stage<Config> for ExtractStage {
    type Input = ParsedDoc;
    type Output = ExtractedDoc;

    fn name(&self) -> &'static str {
        "extract"
    }

    fn run(
        &mut self,
        parsed: ParsedDoc,
        session: &mut Session<Config>,
    ) -> Result<ExtractedDoc, DriverError> {
        let config = session.config();
        let extracted = ac_fields::extract_fields(parsed.value, &*config.dialect)
            .map_err(|e| DriverError::new(e))?;

        if extracted.schemas.is_empty() {
            return Err(DriverError::new("no `$:` section found in input"));
        }

        Ok(ExtractedDoc {
            extracted,
            source_map: parsed.source_map,
            base_path: parsed.base_path,
        })
    }
}
