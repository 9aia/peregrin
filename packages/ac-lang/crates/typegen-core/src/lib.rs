//! # ac-typegen-core
//!
//! Schema-system-agnostic type generation core.
//!
//! The core provides **generic primitives** and **plugin traits** for
//! building type generation contexts from field schema fragments. It has
//! zero knowledge of any AC-specific key (`expr`, `arg`, `merge`,
//! `inheritable`, `mutable`).
//!
//! ## Plugin traits
//!
//! - [`KeyHandler`] — filters and strips special keys from the context.
//!   Implemented by `ac-typegen-key-set-special`.
//! - [`ImportResolver`] — resolves `import()` expressions and merges
//!   imported schemas. Implemented by `ac-typegen-import`.
//! - [`TypegenBackend`] — generates format-specific output.
//!   Implemented by `ac-typegen-backend-json-schema`.
//!
//! The orchestrator (compiler pass, CLI) composes these plugins via
//! trait objects in `Config`.
//!
//! ## Architectural rule
//!
//! > **The core knows nothing about AC keys, the output format, or the filesystem.**
//!
//! ## Dependency graph
//!
//! ```text
//! ac-typegen-core — generic primitives + plugin traits, no AC key knowledge
//! ├── ac-typegen-key-set-special   — KeyHandler impl (knows all special AC keys)
//! ├── ac-typegen-import            — ImportResolver impl (depends on ac-module-resolver-core)
//! └── ac-typegen-backend-json-schema — TypegenBackend impl (JSON Schema)
//!
//! ac-module-resolver-core — ModuleResolver trait (not typegen-specific)
//! ├── ac-module-resolver-file    — FileResolver (filesystem I/O)
//! ├── ac-module-resolver-virtual — VirtualResolver (in-memory)
//! ├── ac-module-resolver-http    — HttpResolver (HTTP via ureq)
//! └── ac-module-resolver-git     — GitResolver (Git via git2)
//! ```

use indexmap::IndexMap;
use serde_json::Value as JsonValue;

///
/// Walks objects and arrays, removing any key in `keys` from object maps.
/// Non-object values are left unchanged. This avoids the deep cloning
/// that the returning variant [`strip_keys`] performs.
pub fn strip_keys_in_place(value: &mut JsonValue, keys: &[&str]) {
    match value {
        JsonValue::Object(map) => {
            map.retain(|key, _| !keys.contains(&key.as_str()));
            for (_, val) in map.iter_mut() {
                strip_keys_in_place(val, keys);
            }
        }
        JsonValue::Array(arr) => {
            for item in arr.iter_mut() {
                strip_keys_in_place(item, keys);
            }
        }
        _ => {}
    }
}

/// Recursively remove the given keys from a JSON value.
///
/// Walks objects and arrays, removing any key in `keys` from object maps.
/// Non-object values are returned unchanged.
///
/// Prefer [`strip_keys_in_place`] when you own the value — it avoids
/// a full deep clone.
pub fn strip_keys(value: &JsonValue, keys: &[&str]) -> JsonValue {
    let mut cloned = value.clone();
    strip_keys_in_place(&mut cloned, keys);
    cloned
}

/// Schema-system-agnostic generation context.
///
/// Holds field name → schema fragment pairs. Built from raw schemas
/// using [`TypegenContext::from_schemas`]. AC-key crates filter and strip
/// the context. Import resolution merges imported schemas. Backends
/// consume the final clean fragments.
pub struct TypegenContext {
    /// Field name → schema fragment.
    ///
    /// Uses `IndexMap` to preserve insertion order while providing O(1)
    /// lookups for `remove_fields` and `merge_fields`.
    pub fields: IndexMap<String, JsonValue>}

impl TypegenContext {
    /// Build from raw field schemas. No filtering, no stripping.
    ///
    /// The orchestrator applies AC-key crates after this.
    pub fn from_schemas(schemas: &[(String, JsonValue)]) -> Self {
        let fields: IndexMap<String, JsonValue> = schemas
            .iter()
            .map(|(name, raw)| (name.clone(), raw.clone()))
            .collect();
        TypegenContext { fields }
    }

    /// Remove fields by name.
    ///
    /// Used by `ac-typegen-merge` to exclude merge fields.
    pub fn remove_fields(&mut self, names: &[String]) {
        self.fields
            .retain(|n, _| !names.iter().any(|name| name == n));
    }

    /// Recursively strip the given keys from all field schemas, in place.
    ///
    /// Used by each AC-key crate to strip its specific key.
    pub fn strip_keys(&mut self, keys: &[&str]) {
        for (_, schema) in &mut self.fields {
            strip_keys_in_place(schema, keys);
        }
    }

    /// Merge additional fields into the context.
    ///
    /// Parent wins — fields already present are not overwritten.
    /// Used by `ac-typegen-import` to merge resolved import schemas.
    pub fn merge_fields(&mut self, imported: &[(String, JsonValue)]) {
        for (name, schema) in imported {
            if !self.fields.contains_key(name) {
                self.fields.insert(name.clone(), schema.clone());
            }
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
pub trait TypegenBackend: Send + Sync {
    type Output;
    /// Generate a single document's type definition.
    fn generate(&self, ctx: &TypegenContext) -> Self::Output;
    /// File extension for emitted files (e.g. "schema.json", "d.ts").
    fn extension(&self) -> &str;
    /// IDE hint comment to inject into source files (e.g.
    /// `# yaml-language-server: $schema={path}`).
    fn ide_hint(&self, schema_rel_path: &str) -> String;
}

mod plugin;

pub use plugin::{ImportResolver, KeyHandler, get_expr};

#[cfg(test)]
mod lib_tests;
