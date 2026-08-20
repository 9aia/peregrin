//! # ac-typegen-mutable
//!
//! Handles the `mutable` AC key for the typegen system.
//!
//! The `mutable` key marks a field as immutable (`mutable: false`) or
//! mutable (`mutable: true`). For type generation, it's simply stripped —
//! the key is AC-specific and not part of any schema vocabulary.
//!
//! Depends on `ac-typegen-core` (core) for generic primitives.

use ac_typegen_core::TypegenContext;

/// The AC key this crate handles.
pub const KEY: &str = "mutable";

/// Strip the `mutable` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext) {
    ctx.strip_keys(&[KEY]);
}

#[cfg(test)]
mod tests;
