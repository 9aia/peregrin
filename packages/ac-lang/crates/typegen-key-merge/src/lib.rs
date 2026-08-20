//! # ac-typegen-merge
//!
//! Handles the `merge` AC key for the typegen system.
//!
//! - **Filtering**: merge fields are excluded from the output schema
//!   (they're removed from data after computation at runtime).
//! - **Stripping**: the `merge` key itself is removed from remaining fields.
//!
//! Depends on `ac-typegen-core` (core) for generic primitives.

use serde_json::Value as JsonValue;

use ac_typegen_core::TypegenContext;

/// The AC key this crate handles.
pub const KEY: &str = "merge";

/// Check if a field schema has `merge: true`.
pub fn is_merge_field(schema: &JsonValue) -> bool {
    schema.get(KEY).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Remove merge fields from the context by name.
///
/// Merge fields are excluded from the output schema — they're spread
/// into the parent scope at runtime and removed.
pub fn filter_fields(ctx: &mut TypegenContext, merge_names: &[String]) {
    ctx.remove_fields(merge_names);
}

/// Strip the `merge` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext) {
    ctx.strip_keys(&[KEY]);
}

#[cfg(test)]
mod tests;
