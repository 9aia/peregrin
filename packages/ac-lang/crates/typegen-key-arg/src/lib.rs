//! # ac-typegen-arg
//!
//! Handles the `arg` AC key for the typegen system.
//!
//! The `arg` key declares a parameter type for a computed field — the
//! field's own data value is available as `${arg}` inside its `expr:`.
//! For type generation, it's stripped — the key is AC-specific and not
//! part of any schema vocabulary.
//!
//! Depends on `ac-typegen-core` (core) for generic primitives.

use serde_json::Value as JsonValue;

use ac_typegen_core::TypegenContext;

/// The AC key this crate handles.
pub const KEY: &str = "arg";

/// Check if a field schema has an `arg` key.
pub fn has_arg(schema: &JsonValue) -> bool {
    schema.get(KEY).is_some()
}

/// Strip the `arg` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext) {
    ctx.strip_keys(&[KEY]);
}

#[cfg(test)]
mod tests;
