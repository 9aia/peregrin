//! # ac-engine-pass-generate
//!
//! Pipeline pass: generate output from the resolved `TypegenContext`.

use ac_engine_core::{Compilation, Config, Pass};

/// Generate output from the resolved `TypegenContext`.
///
/// Requires `ResolvePass` to have run. Uses the configured backend
/// to produce the output schema. Populates `compilation.output`.
pub struct GeneratePass;

impl Pass for GeneratePass {
    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String> {
        let ctx = compilation
            .typegen_ctx
            .take()
            .ok_or("GeneratePass: no typegen context — run ResolvePass first")?;

        let backend = config
            .backend
            .as_ref()
            .ok_or("GeneratePass: no backend configured")?;

        compilation.output = Some(backend.generate(&ctx));
        Ok(())
    }
}

#[cfg(test)]
mod lib_tests;
