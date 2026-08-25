//! # handl-typegen-import
//!
//! Import resolution for the handl typegen system.
//!
//! This crate follows `import()` expressions in merge fields, resolves
//! them via a [`ModuleResolver`], and merges imported schemas into the
//! parent context.
//!
//! It does **not** do any I/O directly — that's delegated to the
//! resolver implementation. This makes import resolution testable
//! without touching the filesystem.
//!
//! Depends on `handl-typegen-core` (for filtering merge fields from
//! imported modules and reading `expr` values to detect `import()` calls).
//!
//! ## Architectural rule
//!
//! > **Import resolution uses a resolver; the resolver does the I/O.**
//!
//! The orchestrator (CLI, build tool) calls them in order:
//!
//! ```ignore
//! // 1. Core: build context from raw schemas
//! let mut ctx = TypegenContext::from_schemas(&extracted.schemas);
//!
//! // 2. handl-key crates: filter + strip (pure)
//! handl_typegen_key_set_special::merge::filter_fields(&mut ctx, &extracted.merge);
//! ctx.strip_keys(&ac_keys);
//!
//! // 3. Import resolution: follow imports, merge (via resolver)
//! handl_typegen_import::apply_imports(
//!     &mut ctx, &extracted, &data, &resolver, &vocab,
//! );
//!
//! // 4. Backend: generate output (pure)
//! let schema = backend.generate(&ctx);
//! ```

mod import_detect;
mod import_resolve;

use std::path::Path;

use handl_dialect::DocumentVocabulary;
use handl_module_resolver_core::ModuleResolver;
use handl_typegen_core::{ImportResolver, TypegenContext};
use serde_json::Value as JsonValue;

pub use import_detect::{ImportKind, detect_import_path, extract_data_path};
pub use import_resolve::resolve_imports;

/// Default import resolver for the handl typegen system.
///
/// Implements [`ImportResolver`] by following `import()` expressions
/// in merge fields, resolving paths via a [`ModuleResolver`], and
/// merging imported schemas into the parent context.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultImportResolver;

impl ImportResolver for DefaultImportResolver {
    fn resolve_imports(
        &self,
        ctx: &mut TypegenContext,
        schemas: &[(String, JsonValue)],
        merge_fields: &[String],
        data: &JsonValue,
        resolver: &dyn ModuleResolver,
        base: Option<&Path>,
        vocab: &dyn DocumentVocabulary,
    ) {
        let resolved = resolve_imports(schemas, merge_fields, data, resolver, base, vocab);
        for (_, imported_fields) in &resolved {
            ctx.merge_fields(imported_fields);
        }
    }
}

/// Apply import resolution to a `TypegenContext`.
///
/// For each merge field with an `import()` expression, follows the import
/// path via `resolver`, extracts the imported module's fields, filters
/// merge fields, strips handl keys, and merges them into the context
/// (parent wins).
///
/// `vocab` provides the reserved key names used for stripping handl-specific
/// keys from imported module schemas.
pub fn apply_imports(
    ctx: &mut TypegenContext,
    schemas: &[(String, JsonValue)],
    merge_fields: &[String],
    data: &JsonValue,
    resolver: &dyn ModuleResolver,
    base: Option<&Path>,
    vocab: &dyn DocumentVocabulary,
) {
    let resolved = resolve_imports(schemas, merge_fields, data, resolver, base, vocab);

    for (_, imported_fields) in &resolved {
        ctx.merge_fields(imported_fields);
    }
}

#[cfg(test)]
mod apply_tests;
#[cfg(test)]
mod import_detect_tests;
#[cfg(test)]
mod import_resolve_tests;
