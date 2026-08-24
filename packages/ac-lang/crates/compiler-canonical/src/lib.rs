//! # ac-compiler-canonical
//!
//! Concrete, canonical compiler — default configurations and preset
//! [`DriverPipeline`](ac_compiler::DriverPipeline) chains wired from the
//! reusable stages in `ac-compiler-common`.
//!
//! ## Presets
//!
//! - `typegen_pipeline` — parse → extract → resolve → generate
//! - `typecheck_pipeline` — parse → extract → typecheck
//! - `full_pipeline` — parse → extract → resolve → typecheck → generate
//!
//! ## Non-pipeline operations
//!
//! `parse`, `eval`, `validate`, and `fields` have their own entry points.

mod resolver;

pub mod eval;
pub mod fields;
pub mod parse;
pub mod typecheck;
pub mod typegen;
pub mod validate;

pub use ac_compiler::*;
pub use ac_compiler_common::*;
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
pub fn typegen_pipeline() -> DriverPipeline<
    Config,
    Then<Then<Then<ParseStage, ExtractStage>, ResolveStage>, GenerateFromResolvedStage>,
> {
    DriverPipeline::new(ParseStage)
        .then(ExtractStage)
        .then(ResolveStage)
        .then(GenerateFromResolvedStage)
}

/// Typecheck pipeline: parse → extract → typecheck.
pub fn typecheck_pipeline()
-> DriverPipeline<Config, Then<Then<ParseStage, ExtractStage>, TypecheckOnlyStage>> {
    DriverPipeline::new(ParseStage)
        .then(ExtractStage)
        .then(TypecheckOnlyStage)
}

/// Full pipeline: parse → extract → resolve → typecheck → generate.
pub fn full_pipeline() -> DriverPipeline<
    Config,
    Then<Then<Then<Then<ParseStage, ExtractStage>, ResolveStage>, TypecheckStage>, GenerateStage>,
> {
    DriverPipeline::new(ParseStage)
        .then(ExtractStage)
        .then(ResolveStage)
        .then(TypecheckStage)
        .then(GenerateStage)
}
