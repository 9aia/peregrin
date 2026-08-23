//! # ac-compiler-common
//!
//! Reusable [`Stage`](ac_compiler::driver::Stage) implementations for the
//! AC compiler pipeline. These are the building blocks that
//! `ac-compiler-canonical` wires into preset pipelines.
//!
//! Use `ac-compiler` for the pipeline abstractions and `ac-compiler-canonical`
//! for the concrete compiler presets.

pub use ac_compiler_pass_extract::ExtractStage;
pub use ac_compiler_pass_generate::{GenerateFromResolvedStage, GenerateStage};
pub use ac_compiler_pass_parse::ParseStage;
pub use ac_compiler_pass_resolve::ResolveStage;
pub use ac_compiler_pass_typecheck::{TypecheckOnlyStage, TypecheckStage};
