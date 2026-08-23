//! # ac-compiler
//!
//! Interface-agnostic operation logic for the AC language.
//!
//! Re-exports the pipeline core from `ac-compiler-core` and provides
//! preset pipelines that compose the built-in passes from
//! `ac-compiler-pass-*` crates.
//!
//! ## Presets
//!
//! - [`Pipeline::typegen`] — parse → extract → resolve → generate
//! - [`Pipeline::typecheck`] — parse → extract → typecheck
//! - [`Pipeline::full`] — parse → extract → resolve → typecheck → generate
//!
//! ## Non-pipeline operations
//!
//! `parse`, `eval`, `validate`, and `fields` have their own entry
//! points — they don't need the pipeline.
//!
//! ## Architectural rule
//!
//! > **The engine knows nothing about how input is read or how output
//! > is presented.** Callers handle I/O; the engine handles logic.

mod resolver;

pub mod eval;
pub mod fields;
pub mod parse;
pub mod typecheck;
pub mod typegen;
pub mod validate;

pub use ac_compiler_core::{
    CheckedDoc, Compilation, Config, Diagnostic, DiagnosticSink, DriverDiagnostic, DriverError,
    DriverPipeline, DriverSeverity, ExtractedDoc, Folder, OptPass, OptPassError, Outcome,
    Pass, PassCategory, PassError, PassGroup, PassManager, PassMetadata, PassRun, ParsedDoc,
    Pipeline, Report, ResolvedDoc, Session, Severity, Source, Stage, Then, TypecheckResult,
    Visitor,
};
pub use resolver::{CompilerHost, Import};

/// Default configuration for CLI usage:
/// - `FileResolver` with `MemoryModuleCache` for filesystem imports
/// - `JsonSchemaInterpreter` for type inference
/// - `JsonSchemaBackend` for JSON Schema generation
pub fn default_cli_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_file::FileResolver::with_cache(
            ac_module_resolver_memory::MemoryModuleCache::new(),
        )),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: Some(Box::new(ac_typegen_backend_json_schema::JsonSchemaBackend)),
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
        extensions: Default::default(),
    }
    .with_extension(ac_typecheck_infer::HostFunctionSigs::default_host())
}

/// Typecheck-only config — no backend needed.
pub fn typecheck_only_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_file::FileResolver::with_cache(
            ac_module_resolver_memory::MemoryModuleCache::new(),
        )),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: None,
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
        extensions: Default::default(),
    }
    .with_extension(ac_typecheck_infer::HostFunctionSigs::default_host())
}

/// Full typegen pipeline: parse → extract → resolve → generate.
pub fn typegen_pipeline(config: Config) -> Pipeline {
    Pipeline::new(config)
        .with_pass(ac_compiler_pass_parse::ParsePass)
        .with_pass(ac_compiler_pass_extract::ExtractPass)
        .with_pass(ac_compiler_pass_resolve::ResolvePass)
        .with_pass(ac_compiler_pass_generate::GeneratePass)
}

/// Typecheck pipeline: parse → extract → typecheck.
pub fn typecheck_pipeline(config: Config) -> Pipeline {
    Pipeline::new(config)
        .with_pass(ac_compiler_pass_parse::ParsePass)
        .with_pass(ac_compiler_pass_extract::ExtractPass)
        .with_pass(ac_compiler_pass_typecheck::TypecheckPass)
}

/// Full pipeline: parse → extract → resolve → typecheck → generate.
/// Use this when you want type safety before code generation.
pub fn full_pipeline(config: Config) -> Pipeline {
    Pipeline::new(config)
        .with_pass(ac_compiler_pass_parse::ParsePass)
        .with_pass(ac_compiler_pass_extract::ExtractPass)
        .with_pass(ac_compiler_pass_resolve::ResolvePass)
        .with_pass(ac_compiler_pass_typecheck::TypecheckPass)
        .with_pass(ac_compiler_pass_generate::GeneratePass)
}

// ---------------------------------------------------------------------------
// driver-lang pipelines — type-checked phase chains
// ---------------------------------------------------------------------------

/// Typegen driver pipeline: `Source → ParsedDoc → ExtractedDoc → ResolvedDoc → JsonValue`.
///
/// Compile-time guarantee that phases connect correctly.
/// Returns a `DriverPipeline<Config, ...>` that you run with a `Session<Config>`.
pub fn typegen_driver() -> DriverPipeline<
    Config,
    Then<
        Then<
            Then<ac_compiler_pass_parse::ParseStage, ac_compiler_pass_extract::ExtractStage>,
            ac_compiler_pass_resolve::ResolveStage,
        >,
        ac_compiler_pass_generate::GenerateFromResolvedStage,
    >,
> {
    DriverPipeline::new(ac_compiler_pass_parse::ParseStage)
        .then(ac_compiler_pass_extract::ExtractStage)
        .then(ac_compiler_pass_resolve::ResolveStage)
        .then(ac_compiler_pass_generate::GenerateFromResolvedStage)
}

/// Typecheck driver pipeline: `Source → ParsedDoc → ExtractedDoc → TypecheckResult`.
///
/// Standalone type checking without import resolution.
pub fn typecheck_driver() -> DriverPipeline<
    Config,
    Then<
        Then<ac_compiler_pass_parse::ParseStage, ac_compiler_pass_extract::ExtractStage>,
        ac_compiler_pass_typecheck::TypecheckOnlyStage,
    >,
> {
    DriverPipeline::new(ac_compiler_pass_parse::ParseStage)
        .then(ac_compiler_pass_extract::ExtractStage)
        .then(ac_compiler_pass_typecheck::TypecheckOnlyStage)
}

/// Full driver pipeline: `Source → ParsedDoc → ExtractedDoc → ResolvedDoc → CheckedDoc → JsonValue`.
///
/// Full compilation with type checking before generation.
pub fn full_driver() -> DriverPipeline<
    Config,
    Then<
        Then<
            Then<
                Then<ac_compiler_pass_parse::ParseStage, ac_compiler_pass_extract::ExtractStage>,
                ac_compiler_pass_resolve::ResolveStage,
            >,
            ac_compiler_pass_typecheck::TypecheckStage,
        >,
        ac_compiler_pass_generate::GenerateStage,
    >,
> {
    DriverPipeline::new(ac_compiler_pass_parse::ParseStage)
        .then(ac_compiler_pass_extract::ExtractStage)
        .then(ac_compiler_pass_resolve::ResolveStage)
        .then(ac_compiler_pass_typecheck::TypecheckStage)
        .then(ac_compiler_pass_generate::GenerateStage)
}
