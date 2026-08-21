//! # ac-compiler-pass-typecheck
//!
//! Pipeline pass: typecheck all AC keys and cross-module imports.

mod runner;

use ac_compiler_core::{Compilation, Config, Pass, PassError};

/// Typecheck all AC keys and cross-module imports.
///
/// Populates `compilation.type_errors` with any type mismatches found.
pub struct TypecheckPass;

impl Pass for TypecheckPass {
    fn should_run(&self, compilation: &Compilation, _config: &Config) -> bool {
        compilation.extracted.is_some()
    }

    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), PassError> {
        runner::run_typecheck(compilation, config)
    }
}

#[cfg(test)]
mod lib_tests;
