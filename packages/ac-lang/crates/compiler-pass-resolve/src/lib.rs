//! # ac-compiler-pass-resolve
//!
//! Reusable `Stage` implementation that resolves schemas and imports
//! into a `ResolvedDoc`.

mod stage;

pub use stage::ResolveStage;