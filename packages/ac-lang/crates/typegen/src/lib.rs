//! # ac-typegen
//!
//! Schema-system-agnostic type generation from AC field declarations.
//!
//! This crate is the **core** of the typegen system. It depends only on
//! `ac-fields` (for `ExtractedFields`) and frontend crates (for parsing
//! imported documents). It does **not** depend on `ac-schema`,
//! `ac-schema-json`, or any schema validation crate.
//!
//! The core performs three operations:
//!
//! 1. **Filter** — exclude merge fields (removed from data after computation).
//! 2. **Strip** — remove AC-specific keys from each field schema.
//! 3. **Resolve imports** — follow `import()` expressions, load imported
//!    documents, extract + strip their fields, and merge their schemas into
//!    the parent (parent wins on conflicts).
//!
//! The output is clean `JsonValue` fragments — the input to all backends.
//!
//! ## Architectural rule
//!
//! > **The core knows nothing about the output format.**
//!
//! Backends (`ac-typegen-json`, future `ac-typegen-typescript`, etc.)
//! implement the [`TypegenBackend`] trait and interpret the clean fragments.

mod strip;
mod import_detect;
mod import_resolve;

use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;

use ac_fields::ExtractedFields;

pub use strip::strip_ac_keys;
pub use import_detect::{detect_import_path, ImportKind};
pub use import_resolve::resolve_imports;

/// AC-specific keys that are stripped from field schemas before generation.
pub const AC_ONLY_KEYS: &[&str] = &["expr", "arg", "merge", "inheritable", "mutable"];

/// Schema-system-agnostic generation context.
///
/// Built from `ExtractedFields` — contains everything a backend needs.
///
/// Merge fields are excluded from `fields`. Their imported schemas are
/// already merged in (parent wins on conflicts). This mirrors the runtime
/// pipeline's final data shape.
pub struct TypegenContext {
    /// Field name → clean schema fragment.
    /// AC keys stripped, merge fields excluded, imported schemas merged in.
    pub fields: Vec<(String, JsonValue)>,
    /// Imported documents that were resolved during context building.
    /// Each entry is (relative_path, generated_schema_fragments).
    /// Backends can use this to generate cross-reference files.
    pub resolved_imports: Vec<(String, Vec<(String, JsonValue)>)>,
    /// Base path for resolving relative imports.
    pub base_path: Option<PathBuf>,
}

impl TypegenContext {
    /// Build from `ExtractedFields` — filters merge fields, strips AC keys,
    /// detects and resolves imports (static and data-driven), merges
    /// imported schemas into the parent (parent wins).
    pub fn from_extracted(
        extracted: &ExtractedFields,
        data: &JsonValue,
        base_path: Option<&Path>,
    ) -> Self {
        let merge_set: std::collections::HashSet<&str> =
            extracted.merge.iter().map(|s| s.as_str()).collect();

        // Start with non-merge fields, stripped of AC keys.
        let mut fields: Vec<(String, JsonValue)> = extracted
            .schemas
            .iter()
            .filter(|(name, _)| !merge_set.contains(name.as_str()))
            .map(|(name, raw)| (name.clone(), strip_ac_keys(raw)))
            .collect();

        // Resolve imports from merge fields and merge into parent.
        let resolved_imports = resolve_imports(
            &extracted.schemas,
            &extracted.merge,
            data,
            base_path,
        );

        // Merge imported schemas into parent (parent wins).
        for (_, imported_fields) in &resolved_imports {
            for (name, schema) in imported_fields {
                if !fields.iter().any(|(n, _)| n == name) {
                    fields.push((name.clone(), schema.clone()));
                }
            }
        }

        TypegenContext {
            fields,
            resolved_imports,
            base_path: base_path.map(|p| p.to_path_buf()),
        }
    }
}

/// A type definition generator. Format-specific.
///
/// The core produces clean `JsonValue` fragments. Each backend interprets
/// them and produces its target format.
///
/// - `ac-typegen-json` → JSON Schema (`JsonValue`)
/// - `ac-typegen-typescript` → TypeScript `.d.ts` (string)
/// - Future: Rust structs, Python TypedDict, etc.
pub trait TypegenBackend {
    type Output;
    /// Generate a single document's type definition.
    fn generate(&self, ctx: &TypegenContext) -> Self::Output;
    /// File extension for emitted files (e.g. "schema.json", "d.ts").
    fn extension(&self) -> &str;
    /// IDE hint comment to inject into source files (e.g.
    /// `# yaml-language-server: $schema={path}`).
    fn ide_hint(&self, schema_rel_path: &str) -> String;
}

// Tests are inline in each module file (strip.rs, import_detect.rs, import_resolve.rs).
// Context tests are in context_test.rs.
#[cfg(test)]
mod context_test;
