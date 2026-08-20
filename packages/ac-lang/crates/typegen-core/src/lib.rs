//! # ac-typegen-core
//!
//! Schema-system-agnostic type generation core.
//!
//! The core provides **generic primitives** for building type generation
//! contexts from field schema fragments. It has zero knowledge of any
//! AC-specific key (`expr`, `arg`, `merge`, `inheritable`, `mutable`).
//!
//! Each AC key has its own crate (`ac-typegen-key-merge`,
//! `ac-typegen-key-inherit`, etc.) that uses these primitives for its
//! specific behavior. The orchestrator composes them.
//!
//! ## Architectural rule
//!
//! > **The core knows nothing about AC keys, the output format, or the filesystem.**
//!
//! ## Dependency graph
//!
//! ```text
//! ac-typegen-core — generic primitives, no AC key knowledge
//! ├── ac-typegen-key-merge       — knows "merge" key
//! ├── ac-typegen-key-inherit     — knows "inheritable" key
//! ├── ac-typegen-key-mutable     — knows "mutable" key
//! ├── ac-typegen-key-expr        — knows "expr" key
//! ├── ac-typegen-key-arg         — knows "arg" key
//! ├── ac-typegen-import          — depends on key-merge + key-expr + ac-resolver-core
//! └── ac-typegen-backend-json-schema — JSON Schema backend
//!
//! ac-resolver-core — ModuleResolver trait (not typegen-specific)
//! ├── ac-resolver-file    — FileResolver (filesystem I/O)
//! ├── ac-resolver-virtual — VirtualResolver (in-memory)
//! └── future: ac-resolver-git, ac-resolver-http
//! ```

use serde_json::Value as JsonValue;

/// Recursively remove the given keys from a JSON value.
///
/// Walks objects and arrays, removing any key in `keys` from object maps.
/// Non-object values are returned unchanged.
pub fn strip_keys(value: &JsonValue, keys: &[&str]) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, val) in map {
                if !keys.contains(&key.as_str()) {
                    cleaned.insert(key.clone(), strip_keys(val, keys));
                }
            }
            JsonValue::Object(cleaned)
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(|v| strip_keys(v, keys)).collect())
        }
        other => other.clone(),
    }
}

/// Schema-system-agnostic generation context.
///
/// Holds field name → schema fragment pairs. Built from raw schemas
/// using [`TypegenContext::from_schemas`]. AC-key crates filter and strip
/// the context. Import resolution merges imported schemas. Backends
/// consume the final clean fragments.
pub struct TypegenContext {
    /// Field name → schema fragment.
    pub fields: Vec<(String, JsonValue)>,
}

impl TypegenContext {
    /// Build from raw field schemas. No filtering, no stripping.
    ///
    /// The orchestrator applies AC-key crates after this.
    pub fn from_schemas(schemas: &[(String, JsonValue)]) -> Self {
        let fields: Vec<(String, JsonValue)> = schemas
            .iter()
            .map(|(name, raw)| (name.clone(), raw.clone()))
            .collect();
        TypegenContext { fields }
    }

    /// Remove fields by name.
    ///
    /// Used by `ac-typegen-merge` to exclude merge fields.
    pub fn remove_fields(&mut self, names: &[String]) {
        self.fields.retain(|(n, _)| !names.iter().any(|name| name == n));
    }

    /// Recursively strip the given keys from all field schemas.
    ///
    /// Used by each AC-key crate to strip its specific key.
    pub fn strip_keys(&mut self, keys: &[&str]) {
        for (_, schema) in &mut self.fields {
            *schema = strip_keys(schema, keys);
        }
    }

    /// Merge additional fields into the context.
    ///
    /// Parent wins — fields already present are not overwritten.
    /// Used by `ac-typegen-import` to merge resolved import schemas.
    pub fn merge_fields(&mut self, imported: &[(String, JsonValue)]) {
        for (name, schema) in imported {
            if !self.fields.iter().any(|(n, _)| n == name) {
                self.fields.push((name.clone(), schema.clone()));
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

#[cfg(test)]
mod tests;
