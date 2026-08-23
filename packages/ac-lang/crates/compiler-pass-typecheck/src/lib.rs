//! # ac-compiler-pass-typecheck
//!
//! Reusable `Stage` implementations that typecheck `ExtractedDoc` or
//! `ResolvedDoc`.

mod stage;

pub use stage::{TypecheckOnlyStage, TypecheckStage};
