//! # ac-typegen-import
//!
//! Import resolution for the AC typegen system.
//!
//! This crate follows `import()` expressions in merge fields, resolves
//! them via a [`ModuleResolver`], and merges imported schemas into the
//! parent context.
//!
//! It does **not** do any I/O directly — that's delegated to the
//! resolver implementation. This makes import resolution testable
//! without touching the filesystem.
//!
//! Depends on `ac-typegen-key-merge` (for filtering merge fields from
//! imported modules) and `ac-typegen-key-expr` (for reading `expr`
//! values to detect `import()` calls).
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
//! // 2. AC-key crates: filter + strip (pure)
//! ac_typegen_key_merge::filter_fields(&mut ctx, &extracted.merge);
//! ctx.strip_keys(&ac_keys);
//!
//! // 3. Import resolution: follow imports, merge (via resolver)
//! ac_typegen_import::apply_imports(
//!     &mut ctx, &extracted, &data, &resolver, &ac_keys,
//! );
//!
//! // 4. Backend: generate output (pure)
//! let schema = backend.generate(&ctx);
//! ```

mod import_detect;
mod import_resolve;

use std::path::Path;

use serde_json::Value as JsonValue;

use ac_resolver_core::ModuleResolver;
use ac_typegen_core::TypegenContext;

pub use import_detect::{detect_import_path, extract_data_path, ImportKind};
pub use import_resolve::resolve_imports;

/// Apply import resolution to a `TypegenContext`.
///
/// For each merge field with an `import()` expression, follows the import
/// path via `resolver`, extracts the imported module's fields, filters
/// merge fields, strips AC keys, and merges them into the context
/// (parent wins).
///
/// `ac_keys` is the list of all AC-specific keys to strip from imported
/// modules. The orchestrator collects these from all AC-key crates.
pub fn apply_imports(
    ctx: &mut TypegenContext,
    schemas: &[(String, JsonValue)],
    merge_fields: &[String],
    data: &JsonValue,
    resolver: &dyn ModuleResolver,
    base: Option<&Path>,
    ac_keys: &[&str],
) {
    let resolved = resolve_imports(schemas, merge_fields, data, resolver, base, ac_keys);

    for (_, imported_fields) in &resolved {
        ctx.merge_fields(imported_fields);
    }
}

#[cfg(test)]
mod import_detect_tests;
#[cfg(test)]
mod import_resolve_tests;
#[cfg(test)]
mod apply_tests;
