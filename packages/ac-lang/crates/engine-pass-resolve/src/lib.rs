//! # ac-engine-pass-resolve
//!
//! Pipeline pass: resolve imports and build a `TypegenContext`.

use ac_engine_core::{Compilation, Config, Pass};

/// Resolve imports and build a `TypegenContext`.
///
/// 1. Builds a `TypegenContext` from extracted schemas.
/// 2. Filters merge fields.
/// 3. Strips all AC keys.
/// 4. Resolves imports via the configured `ModuleResolver`.
pub struct ResolvePass;

impl Pass for ResolvePass {
    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String> {
        let extracted = compilation
            .extracted
            .as_ref()
            .ok_or("ResolvePass: no extracted fields — run ExtractPass first")?;

        let ac_keys: &[&str] = &[
            ac_typegen_key_merge::KEY,
            ac_typegen_key_inherit::KEY,
            ac_typegen_key_mutable::KEY,
            ac_typegen_key_expr::KEY,
            ac_typegen_key_arg::KEY,
        ];

        let data = extracted.data.inner().clone();

        let mut ctx = ac_typegen_core::TypegenContext::from_schemas(&extracted.schemas);
        ac_typegen_key_merge::filter_fields(&mut ctx, &extracted.merge);
        ctx.strip_keys(ac_keys);

        let base = compilation.source.base_path();
        ac_typegen_import::apply_imports(
            &mut ctx,
            &extracted.schemas,
            &extracted.merge,
            &data,
            config.resolver.as_ref(),
            base.as_deref(),
            ac_keys,
        );

        compilation.typegen_ctx = Some(ctx);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
