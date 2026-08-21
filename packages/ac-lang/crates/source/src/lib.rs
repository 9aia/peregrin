//! # ac-source
//!
//! Source location tracking for the AC language.
//!
//! Provides [`SourceLocation`] (1-indexed line:column) and [`SourceMap`]
//! (maps dotted paths to locations). Both are pure data structures —
//! format-specific scanning lives in each frontend crate.
//!
//! ## Architectural rule
//!
//! > **`ac-source` is format-agnostic.** Each frontend builds a
//! > `SourceMap` using its own format-specific knowledge (indentation,
//! > braces, sections, etc.).

mod location;
mod scanner;
mod sourcemap;

pub use location::SourceLocation;
pub use scanner::{KeyFound, KeyMatcher, LineIndex, leading_whitespace, walk_value};
pub use sourcemap::SourceMap;

#[cfg(test)]
mod lib_tests;
