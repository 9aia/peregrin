use indexmap::IndexMap;
use serde_json::Value as JsonValue;

use crate::strip::strip_keys_in_place;

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
        self.fields.retain(|n, _| !names.iter().any(|name| name == n));
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