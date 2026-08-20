//! # ac-typegen-expr
//!
//! Handles the `expr` AC key for the typegen system.
//!
//! The `expr` key declares a computed expression for a field. For type
//! generation, it's stripped — the expression is AC-specific and not
//! part of any schema vocabulary.
//!
//! `ac-typegen-import` depends on this crate to read `expr` values for
//! detecting `import()` calls.
//!
//! Depends on `ac-typegen-core` (core) for generic primitives.

use serde_json::Value as JsonValue;

use ac_typegen_core::TypegenContext;

/// The AC key this crate handles.
pub const KEY: &str = "expr";

/// Get the expression string from a field schema, if present.
pub fn get_expr(schema: &JsonValue) -> Option<String> {
    schema.get(KEY).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Strip the `expr` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext) {
    ctx.strip_keys(&[KEY]);
}

#[cfg(test)]
mod lib_tests;
