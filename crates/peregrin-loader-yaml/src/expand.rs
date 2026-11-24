//! Document-merge helpers — the `deepMerge`/`dedupe_node_ids`/
//! `merge_documents` trio that stayed behind when the expansion passes
//! (`include:`/`snippet:`/`${...}`/`tamias:`) moved to
//! `scarab-compose`'s [`Composer`](scarab_compose::Composer).
//!
//! [`deep_merge`] mirrors the TS `deepMerge(local, imported)`: local
//! wins; dicts merge per-key, arrays concatenate local-first, scalars
//! take local.

use std::collections::HashSet;

use serde_json::{Map, Value};

/// TS `deepMerge(local, imported)` — local wins; dicts merge per-key,
/// arrays concatenate local-first, scalars take local.
pub fn deep_merge(local: &Value, imported: &Value) -> Value {
    match (local, imported) {
        (Value::Object(local_map), Value::Object(imported_map)) => {
            let mut result = imported_map.clone();
            for (key, value) in local_map {
                let merged = match result.get(key) {
                    Some(existing) => deep_merge(value, existing),
                    None => value.clone(),
                };
                result.insert(key.clone(), merged);
            }
            Value::Object(result)
        }
        (Value::Array(local_items), Value::Array(imported_items)) => {
            let mut out = local_items.clone();
            out.extend(imported_items.iter().cloned());
            Value::Array(out)
        }
        _ => local.clone(),
    }
}

/// Node-id dedupe — `deep_merge` concatenates arrays local-first, so
/// an include or overlay that redefines a node `id` would land it
/// twice in `nodes:` and the engine would fire it twice. Keeping the
/// *first* occurrence keeps the local definition (local items sort
/// first after the concat) and the earlier include on a
/// base-vs-base collision. Non-node items and nodes without a
/// string `id` pass through untouched.
pub fn dedupe_node_ids(raw: &mut Map<String, Value>) {
    let Some(Value::Array(nodes)) = raw.get_mut("nodes") else {
        return;
    };
    let mut seen = HashSet::new();
    nodes.retain(|node| {
        let Some(id) = node.get("id").and_then(Value::as_str) else {
            return true;
        };
        seen.insert(id.to_string())
    });
}

/// Document merge for bundle overlay composition — [`deep_merge`]
/// (local wins, arrays local-first) plus [`dedupe_node_ids`] so a
/// redefined node `id` merges rather than double-firing.
#[must_use]
pub fn merge_documents(local: &Value, imported: &Value) -> Value {
    let mut merged = deep_merge(local, imported);
    if let Value::Object(map) = &mut merged {
        dedupe_node_ids(map);
    }
    merged
}
