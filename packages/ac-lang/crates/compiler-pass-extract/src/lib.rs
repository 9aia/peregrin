//! # ac-compiler-pass-extract
//!
//! Pipeline pass: extract fields, functions, and data from parsed document.

use ac_compiler_core::{Compilation, Config, Pass, PassError};
use ac_dialect_canonical::AC_DIALECT;

/// Extract fields, functions, and data from the parsed document.
pub struct ExtractPass;

impl Pass for ExtractPass {
    fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), PassError> {
        let parsed = compilation
            .parsed
            .take()
            .ok_or_else(|| PassError::new("extract", "no parsed value — run ParsePass first"))?;

        let extracted = ac_fields::extract_fields(parsed, &AC_DIALECT)
            .map_err(|e| PassError::new("extract", e))?;

        if extracted.schemas.is_empty() {
            return Err(PassError::new("extract", "no `$:` section found in input"));
        }

        compilation.extracted = Some(extracted);
        Ok(())
    }
}

#[cfg(test)]
mod lib_tests;
