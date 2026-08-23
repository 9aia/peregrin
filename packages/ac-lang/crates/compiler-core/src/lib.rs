//! # ac-compiler-core
//!
//! Pipeline core — [`Compilation`], [`Source`], [`Config`], [`Pass`] trait,
//! [`Pipeline`] builder, [`PassGroup`], [`Diagnostic`] system, and
//! [`Visitor`]/[`Folder`] infrastructure.
//!
//! Pass implementations live in separate crates:
//!
//! - `ac-compiler-pass-parse` — `ParsePass`
//! - `ac-compiler-pass-extract` — `ExtractPass`
//! - `ac-compiler-pass-resolve` — `ResolvePass`
//! - `ac-compiler-pass-typecheck` — `TypecheckPass`
//! - `ac-compiler-pass-generate` — `GeneratePass`
//!
//! Custom passes (linting, optimization, JIT) implement [`Pass`] and
//! slot into the pipeline without core changes.

mod compilation;
mod config;
mod diagnostic;
pub mod driver;
mod error;
mod pass_meta;
mod pipeline;
mod visitor;

pub use ac_dialect::{Dialect, DocumentVocabulary};
pub use compilation::{Compilation, Source};
pub use config::Config;
pub use diagnostic::{Diagnostic, DiagnosticSink, Severity};
pub use error::PassError;
pub use pass_meta::{PassCategory, PassMetadata};
pub use pipeline::{Pass, PassGroup, Pipeline};
pub use visitor::{Folder, Visitor, walk_expr, walk_extracted, walk_field, fold_expr, fold_field};

// New framework-backed types
pub use driver::{
    CheckedDoc, DriverDiagnostic, DriverError, DriverPipeline, DriverSeverity, ExtractedDoc,
    OptPass, OptPassError, Outcome, ParsedDoc, PassManager, PassRun, Report, ResolvedDoc,
    Session, Stage, Then, TypecheckResult};

#[cfg(test)]
mod compilation_tests;

#[cfg(test)]
mod pipeline_tests;
