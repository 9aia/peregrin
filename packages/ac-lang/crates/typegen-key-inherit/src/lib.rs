//! # ac-typegen-inherit
//!
//! Handles the `inheritable` AC key for the typegen system.
//!
//! The `inheritable` key marks a field as propagating its value into
//! nested objects. For type generation, it's simply stripped — the
//! key is AC-specific and not part of any schema vocabulary.
//!
//! Depends on `ac-typegen-core` (core) for generic primitives.

use ac_typegen_core::TypegenContext;

/// The AC key this crate handles.
pub const KEY: &str = "inheritable";

/// Strip the `inheritable` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext) {
    ctx.strip_keys(&[KEY]);
}

#[cfg(test)]
mod lib_tests;
