//! driver-lang Stage implementations for the generate phase.

use handl_compiler::Config;
use handl_compiler::driver::{CheckedDoc, DriverError, ResolvedDoc, Session, Stage};
use serde_json::Value as JsonValue;

/// Generate phase: `CheckedDoc → JsonValue`.
///
/// Generates output from the resolved and typechecked artifacts.
pub struct GenerateStage;

impl Stage<Config> for GenerateStage {
    type Input = CheckedDoc;
    type Output = JsonValue;

    fn name(&self) -> &'static str {
        "generate"
    }

    fn run(
        &mut self,
        doc: CheckedDoc,
        session: &mut Session<Config>,
    ) -> Result<JsonValue, DriverError> {
        let config = session.config();
        let backend = config
            .backend
            .as_ref()
            .ok_or_else(|| DriverError::new("no backend configured"))?;

        Ok(backend.generate(&doc.typegen_ctx))
    }
}

/// Generate phase (skip typecheck): `ResolvedDoc → JsonValue`.
///
/// For the typegen-only pipeline where typecheck is not needed.
pub struct GenerateFromResolvedStage;

impl Stage<Config> for GenerateFromResolvedStage {
    type Input = ResolvedDoc;
    type Output = JsonValue;

    fn name(&self) -> &'static str {
        "generate"
    }

    fn run(
        &mut self,
        doc: ResolvedDoc,
        session: &mut Session<Config>,
    ) -> Result<JsonValue, DriverError> {
        let config = session.config();
        let backend = config
            .backend
            .as_ref()
            .ok_or_else(|| DriverError::new("no backend configured"))?;

        Ok(backend.generate(&doc.typegen_ctx))
    }
}
