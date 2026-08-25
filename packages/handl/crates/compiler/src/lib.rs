//! # handl-compiler
//!
//! Compiler pipeline abstractions.
//!
//! This crate exports the typed pipeline primitives from `driver-lang`
//! (`Stage`, `Session`, `Pipeline`, `Then`), the compiler `Config`, the
//! `Source` type, and the artifact types (`ParsedDoc`, `ExtractedDoc`, etc.).
//!
//! Concrete preset pipelines and default configurations live in
//! `handl-compiler-canonical`; reusable `Stage` implementations live in
//! `handl-compiler-common`.

mod config;
mod diagnostic;
pub mod driver;
mod source;
pub mod visitor;

pub use handl_dialect::{Dialect, DocumentVocabulary};
pub use config::Config;
pub use diagnostic::{Diagnostic, DiagnosticSink, Severity};
pub use driver::{
    CheckedDoc, DriverDiagnostic, DriverError, DriverPipeline, DriverSeverity, ExtractedDoc,
    ParsedDoc, ResolvedDoc, Session, Stage, Then, TypecheckResult,
};
pub use source::Source;
pub use visitor::{Folder, Visitor, fold_expr, fold_field, walk_expr, walk_extracted, walk_field};
