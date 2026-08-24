//! # ac-compiler-pass-generate
//!
//! Reusable `Stage` implementations that generate output from a
//! `ResolvedDoc` or `CheckedDoc`.

mod stage;

pub use stage::{GenerateFromResolvedStage, GenerateStage};