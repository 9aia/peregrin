//! # ac-compiler-pass-parse
//!
//! Pipeline pass: parse source text into a `NonFunctionValue`.

use ac_compiler_core::{Compilation, Config, Pass, PassError};

/// Parse the source text into a `NonFunctionValue`.
pub struct ParsePass;

impl Pass for ParsePass {
    fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), PassError> {
        let frontend = ac_frontend::frontend_for(&compilation.source.format)
            .map_err(|e| PassError::new("parse", e))?;
        let (value, source_map) = frontend
            .parse_with_locations(&compilation.source.input)
            .map_err(|e| PassError::new("parse", e))?;
        compilation.parsed = Some(value);
        compilation.source_map = source_map;
        Ok(())
    }
}

#[cfg(test)]
mod lib_tests;
