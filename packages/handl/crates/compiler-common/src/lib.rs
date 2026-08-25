//! # handl-compiler-common
//!
//! Reusable [`Stage`](handl_compiler::driver::Stage) implementations for the
//! handl compiler pipeline. These are the building blocks that
//! `handl-compiler-canonical` wires into preset pipelines.
//!
//! Use `handl-compiler` for the pipeline abstractions and `handl-compiler-canonical`
//! for the concrete compiler presets.

pub use handl_compiler_pass_extract::ExtractStage;
pub use handl_compiler_pass_generate::{GenerateFromResolvedStage, GenerateStage};
pub use handl_compiler_pass_parse::ParseStage;
pub use handl_compiler_pass_resolve::ResolveStage;
pub use handl_compiler_pass_typecheck::{TypecheckOnlyStage, TypecheckStage};
