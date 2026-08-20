//! # ac-engine-pass-parse
//!
//! Pipeline pass: parse source text into a `RuntimeValue`.

use ac_engine_core::{Compilation, Config, Pass};

/// Parse the source text into a `RuntimeValue`.
pub struct ParsePass;

impl Pass for ParsePass {
    fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
        let frontend = ac_frontend::frontend_for(&compilation.source.format)?;
        let (value, source_map) = frontend.parse_with_locations(&compilation.source.input)?;
        compilation.parsed = Some(value);
        compilation.source_map = source_map;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
