//! # ac-engine-core
//!
//! Pipeline core — [`Compilation`], [`Source`], [`Config`], [`Pass`] trait,
//! and [`Pipeline`] builder.
//!
//! Pass implementations live in separate crates:
//!
//! - `ac-engine-pass-parse` — `ParsePass`
//! - `ac-engine-pass-extract` — `ExtractPass`
//! - `ac-engine-pass-resolve` — `ResolvePass`
//! - `ac-engine-pass-typecheck` — `TypecheckPass`
//! - `ac-engine-pass-generate` — `GeneratePass`
//!
//! Custom passes (linting, optimization, JIT) implement [`Pass`] and
//! slot into the pipeline without core changes.

mod compilation;
mod config;
mod pipeline;

pub use compilation::{Compilation, Source};
pub use config::Config;
pub use pipeline::{Pass, Pipeline};

#[cfg(test)]
mod compilation_tests;

#[cfg(test)]
mod pipeline_tests;
