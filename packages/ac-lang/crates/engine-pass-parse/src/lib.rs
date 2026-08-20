//! # ac-engine-pass-parse
//!
//! Pipeline pass: parse source text into a `RuntimeValue`.

use ac_engine_core::{Compilation, Config, Pass};

/// Parse the source text into a `RuntimeValue`.
pub struct ParsePass;

impl Pass for ParsePass {
    fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
        let frontend = ac_frontend::frontend_for(&compilation.source.format)?;
        let value = frontend.parse(&compilation.source.input)?;
        compilation.parsed = Some(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
