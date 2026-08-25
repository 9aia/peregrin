//! Front-end phase pipeline backed by [`driver_lang`].
//!
//! Each compilation phase is a [`Stage<Config>`] with typed `Input` and
//! `Output` artifacts. The compiler enforces at compile time that phases
//! connect correctly — you cannot wire a generate stage after a parse
//! stage because the output type of parse does not match the input type
//! of generate.
//!
//! ## Artifact flow
//!
//! ```text
//! Source → ParsedDoc → ExtractedDoc → ResolvedDoc → CheckedDoc → JsonValue
//! ```
//!
//! ## Preset pipelines
//!
//! - **typegen**: `Source → ParsedDoc → ExtractedDoc → ResolvedDoc → JsonValue`
//! - **typecheck**: `Source → ParsedDoc → ExtractedDoc → TypecheckResult`
//! - **full**: `Source → ParsedDoc → ExtractedDoc → ResolvedDoc → CheckedDoc → JsonValue`
//!
//! ## Stage implementations
//!
//! Stage impls live in the pass crates:
//!
//! - `handl-compiler-pass-parse` — `ParseStage`
//! - `handl-compiler-pass-extract` — `ExtractStage`
//! - `handl-compiler-pass-resolve` — `ResolveStage`
//! - `handl-compiler-pass-typecheck` — `TypecheckStage`, `TypecheckOnlyStage`
//! - `handl-compiler-pass-generate` — `GenerateStage`, `GenerateFromResolvedStage`
//!
//! ## Custom stages
//!
//! ```ignore
//! use driver_lang::{Pipeline, Session, Stage};
//! use handl_compiler::driver::*;
//!
//! struct MyLintStage;
//! impl Stage<Config> for MyLintStage {
//!     type Input = ExtractedDoc;
//!     type Output = ExtractedDoc;
//!     fn name(&self) -> &'static str { "lint" }
//!     fn run(&mut self, input: ExtractedDoc, session: &mut Session<Config>)
//!         -> Result<ExtractedDoc, DriverError>
//!     {
//!         // inspect input.extracted, emit warnings via session
//!         Ok(input) // pass through
//!     }
//! }
//!
//! let pipeline = Pipeline::new(ParseStage)
//!     .then(ExtractStage)
//!     .then(MyLintStage)
//!     .then(ResolveStage)
//!     .then(GenerateStage);
//! ```

use handl_vm_values_json::NonFunctionValue;
use std::path::PathBuf;

use handl_fields::ExtractedFields;
use handl_source::SourceMap;
use handl_typecheck_core::TypeError;
use handl_typegen_core::TypegenContext;

// Re-export driver-lang types
pub use driver_lang::{
    Diagnostic as DriverDiagnostic, DriverError, Pipeline as DriverPipeline, Session,
    Severity as DriverSeverity, Stage, Then,
};

// ---------------------------------------------------------------------------
// Artifact types — typed inputs/outputs for each phase
// ---------------------------------------------------------------------------

/// Output of the parse phase — a parsed document value plus source map.
pub struct ParsedDoc {
    pub value: NonFunctionValue,
    pub source_map: SourceMap,
    /// Base path for relative import resolution (parent dir of source file).
    pub base_path: Option<PathBuf>,
    /// Source format used to parse the document (e.g. `yaml`, `json`, `toml`, `handl`).
    pub format: String,
}

/// Output of the extract phase — extracted fields plus carried source map.
pub struct ExtractedDoc {
    pub extracted: ExtractedFields,
    pub source_map: SourceMap,
    /// Base path for relative import resolution.
    pub base_path: Option<PathBuf>,
}

/// Output of the resolve phase — extracted fields, typegen context, source map.
pub struct ResolvedDoc {
    pub extracted: ExtractedFields,
    pub typegen_ctx: TypegenContext,
    pub source_map: SourceMap,
    /// Base path — carried through for typecheck import resolution.
    pub base_path: Option<PathBuf>,
}

/// Output of the typecheck phase (after resolve) — all artifacts plus type errors.
pub struct CheckedDoc {
    pub extracted: ExtractedFields,
    pub typegen_ctx: TypegenContext,
    pub type_errors: Vec<TypeError>,
    pub source_map: SourceMap,
}

/// Output of the typecheck-only phase (no resolve) — extracted fields + errors.
pub struct TypecheckResult {
    pub extracted: ExtractedFields,
    pub type_errors: Vec<TypeError>,
    pub source_map: SourceMap,
}
