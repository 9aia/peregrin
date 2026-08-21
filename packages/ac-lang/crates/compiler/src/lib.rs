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

mod expr_eval;
mod resolver;

pub mod parse;
pub mod eval;
pub mod validate;
pub mod fields;
pub mod typegen;
pub mod typecheck;

pub use ac_compiler_core::{Compilation, Config, Pass, Pipeline, Source};
pub use expr_eval::ExprEval;
pub use resolver::VmResolver;

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
        backend: Some(Box::new(
            ac_typegen_backend_json_schema::JsonSchemaBackend,
        )),
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
    }
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
    }
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
