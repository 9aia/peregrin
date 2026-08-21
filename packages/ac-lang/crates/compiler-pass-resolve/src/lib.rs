//! # ac-compiler-pass-resolve
//!
//! Pipeline pass: resolve imports and build a `TypegenContext`.

use ac_compiler_core::{Compilation, Config, Pass};

/// Resolve imports and build a `TypegenContext`.
///
/// 1. Builds a `TypegenContext` from extracted schemas.
/// 2. Filters merge fields via the configured `KeyHandler`.
/// 3. Strips all special keys via the configured `KeyHandler`.
/// 4. Resolves imports via the configured `ImportResolver`.
pub struct ResolvePass;

impl Pass for ResolvePass {
    fn should_run(&self, compilation: &Compilation, _config: &Config) -> bool {
        compilation.extracted.is_some()
    }

    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String> {
        let extracted = compilation
            .extracted
            .as_ref()
            .ok_or("ResolvePass: no extracted fields — run ExtractPass first")?;

        let vocab = config.dialect.document_vocabulary();

        let data = extracted.data.inner();

        let mut ctx = ac_typegen_core::TypegenContext::from_schemas(&extracted.schemas);
        config.key_handler.filter_fields(&mut ctx, &extracted.merge);
        config.key_handler.strip_keys(&mut ctx, vocab);

        let base = compilation.source.base_path();
        config.import_resolver.resolve_imports(
            &mut ctx,
            &extracted.schemas,
            &extracted.merge,
            data,
            config.resolver.as_ref(),
            base.as_deref(),
            vocab,
        );

        compilation.typegen_ctx = Some(ctx);
        Ok(())
    }
}

#[cfg(test)]
mod lib_tests;
