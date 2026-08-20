//! # ac-source
//!
//! Source location tracking for the AC language.
//!
//! Provides [`SourceLocation`] (1-indexed line:column) and [`SourceMap`]
//! (resolves dotted paths to locations in source text).
//!
//! Zero dependencies — works with any format (YAML, JSON, TOML) by
//! scanning the raw source text for key names.
//!
//! ## Architectural rule
//!
//! > **Source locations are resolved post-hoc, not carried through the
//! > value tree.** Error types carry an optional `SourceLocation` that
//! > is filled in by the engine after a pass produces errors, using the
//! > original source text.

mod sourcemap;

pub use sourcemap::{SourceLocation, SourceMap};

#[cfg(test)]
mod tests;
