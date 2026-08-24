//! # ac-typegen-key-set-special
//!
//! Typegen key handling for the special AC key set.
//!
//! Implements the [`KeyHandler`] trait from `ac-typegen-core`. Each
//! canonical AC key (`arg`, `expr`, `inheritable`, `merge`, `mutable`)
//! has its own module within this crate. The modules are thin — they define
//! a `KEY` constant and a `strip_key` function that delegates to
//! [`TypegenContext::strip_keys`]. Keys with additional behavior (`expr`,
//! `merge`, `arg`) also expose helper functions.
//!
//! Depends on `ac-typegen-core` for generic primitives.

use ac_dialect::DocumentVocabulary;
use ac_typegen_core::{KeyHandler, TypegenContext};

pub mod arg;
pub mod expr;
pub mod inheritable;
pub mod merge;
pub mod mutable;

/// Key handler for the standard AC special key set.
///
/// Implements [`KeyHandler`] using the 5 canonical AC keys (`arg`, `expr`,
/// `inheritable`, `merge`, `mutable`). Key names are resolved via
/// [`DocumentVocabulary`], so custom dialects with different key names
/// work without changes.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpecialKeyHandler;

impl KeyHandler for SpecialKeyHandler {
    fn filter_fields(&self, ctx: &mut TypegenContext, merge_names: &[String]) {
        ctx.remove_fields(merge_names);
    }

    fn strip_keys(&self, ctx: &mut TypegenContext, vocab: &dyn DocumentVocabulary) {
        let keys: Vec<&str> = vocab.reserved_field_keys();
        ctx.strip_keys(&keys);
    }
}

#[cfg(test)]
mod arg_tests;

#[cfg(test)]
mod expr_tests;

#[cfg(test)]
mod inheritable_tests;

#[cfg(test)]
mod merge_tests;

#[cfg(test)]
mod mutable_tests;
