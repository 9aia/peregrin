//! # ac-engine-pass-extract
//!
//! Pipeline pass: extract fields, functions, and data from parsed document.

use ac_engine_core::{Compilation, Config, Pass};

/// Extract fields, functions, and data from the parsed document.
pub struct ExtractPass;

impl Pass for ExtractPass {
    fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
        let parsed = compilation
            .parsed
            .take()
            .ok_or("ExtractPass: no parsed value — run ParsePass first")?;

        let extracted = ac_fields::extract_fields(parsed)?;

        if extracted.schemas.is_empty() {
            return Err("no `fields:` section found in input".to_string());
        }

        compilation.extracted = Some(extracted);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
