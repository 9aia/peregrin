//! driver-lang Stage implementation for the resolve phase.

use ac_compiler::Config;
use ac_compiler::driver::{DriverError, ExtractedDoc, ResolvedDoc, Session, Stage};

/// Resolve phase: `ExtractedDoc → ResolvedDoc`.
///
/// 1. Builds a `TypegenContext` from extracted schemas.
/// 2. Filters merge fields via the configured `KeyHandler`.
/// 3. Strips all special keys via the configured `KeyHandler`.
/// 4. Resolves imports via the configured `ImportResolver`.
pub struct ResolveStage;

impl Stage<Config> for ResolveStage {
    type Input = ExtractedDoc;
    type Output = ResolvedDoc;

    fn name(&self) -> &'static str {
        "resolve"
    }

    fn run(
        &mut self,
        doc: ExtractedDoc,
        session: &mut Session<Config>,
    ) -> Result<ResolvedDoc, DriverError> {
        let config = session.config();
        let vocab = config.dialect.document_vocabulary();
        let extracted = &doc.extracted;

        let mut ctx = ac_typegen_core::TypegenContext::from_schemas(&extracted.schemas);
        config.key_handler.filter_fields(&mut ctx, &extracted.merge);
        config.key_handler.strip_keys(&mut ctx, vocab);

        let data = extracted.data.inner();

        config.import_resolver.resolve_imports(
            &mut ctx,
            &extracted.schemas,
            &extracted.merge,
            data,
            config.resolver.as_ref(),
            doc.base_path.as_deref(),
            vocab,
        );

        Ok(ResolvedDoc {
            extracted: doc.extracted,
            typegen_ctx: ctx,
            source_map: doc.source_map,
            base_path: doc.base_path,
        })
    }
}
