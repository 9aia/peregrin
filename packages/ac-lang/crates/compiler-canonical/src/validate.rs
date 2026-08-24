use ac_dialect::{Dialect, Schema};
use ac_dialect_canonical::AC_DIALECT;
use serde_json::Value;

use crate::{
    Config, DriverError, DriverPipeline, ParseStage, ParsedDoc, Session, Source, Stage,
    default_cli_config,
};

/// Validate phase: `ParsedDoc → ()`.
///
/// Validates the parsed document against a supplied JSON Schema using
/// the canonical AC dialect.
pub struct ValidateStage {
    pub schema: Value,
}

impl Stage<Config> for ValidateStage {
    type Input = ParsedDoc;
    type Output = ();

    fn name(&self) -> &'static str {
        "validate"
    }

    fn run(&mut self, doc: ParsedDoc, _session: &mut Session<Config>) -> Result<(), DriverError> {
        let vocab = AC_DIALECT.schema_vocabulary();
        let compiled = vocab
            .compile(&self.schema)
            .map_err(|e| DriverError::new(format!("failed to compile schema: {e}")))?;

        compiled.validate(&doc.value).map_err(|e| {
            let location = doc.source_map.resolve_path(&e.path);
            let msg = match location {
                Some(loc) => format!("[{loc}] {e}"),
                None => e.to_string(),
            };
            DriverError::new(msg)
        })
    }
}

/// Validate an AC document through the parse → validate pipeline.
pub fn run(input: &str, format: &str, schema_str: &str) -> Result<(), String> {
    let schema: Value = serde_json::from_str(schema_str)
        .map_err(|e| format!("failed to parse schema JSON: {e}"))?;
    let source = Source::new(input, format);
    let mut pipeline = DriverPipeline::new(ParseStage).then(ValidateStage { schema });
    let mut session = Session::new(default_cli_config());
    pipeline
        .run(source, &mut session)
        .map_err(|e| e.to_string())
}
