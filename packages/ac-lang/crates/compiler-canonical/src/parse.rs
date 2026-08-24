use ac_frontend::YamlFrontend;
use ac_runtime_values::Frontend;

use crate::{DriverPipeline, Session, Source, default_cli_config, ParseStage};

/// Parse a file through the parse pipeline and serialize it back to YAML.
pub fn run(input: &str, format: &str) -> Result<String, String> {
    let source = Source::new(input, format);
    let mut pipeline = DriverPipeline::new(ParseStage);
    let mut session = Session::new(default_cli_config());
    let parsed = pipeline
        .run(source, &mut session)
        .map_err(|e| e.to_string())?;
    YamlFrontend.serialize(&parsed.value).map_err(|e| e.to_string())
}
