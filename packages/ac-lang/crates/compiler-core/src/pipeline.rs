//! Pass trait and Pipeline builder.
//!
//! The pipeline is a sequence of [`Pass`]es that run in order, each
//! receiving `&mut Compilation` and `&Config`. Passes are the extension
//! point — custom passes (linting, optimization, JIT) can be added
//! without modifying the core.
//!
//! Built-in pass implementations live in separate crates:
//!
//! - `ac-compiler-pass-parse` — `ParsePass`
//! - `ac-compiler-pass-extract` — `ExtractPass`
//! - `ac-compiler-pass-resolve` — `ResolvePass`
//! - `ac-compiler-pass-typecheck` — `TypecheckPass`
//! - `ac-compiler-pass-generate` — `GeneratePass`
//!
//! Presets (typegen, typecheck, full) are provided by `ac-compiler`,
//! which depends on all pass crates. This crate stays pass-agnostic.
//!
//! ## Custom passes
//!
//! ```ignore
//! struct LintPass;
//! impl Pass for LintPass {
//!     fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String> {
//!         // read compilation.extracted, write lint warnings
//!         Ok(())
//!     }
//! }
//!
//! let pipeline = Pipeline::new(Config::default_cli())
//!     .with_pass(ParsePass)
//!     .with_pass(ExtractPass)
//!     .with_pass(LintPass)      // custom — no core changes
//!     .with_pass(TypecheckPass)
//!     .with_pass(GeneratePass);
//! ```

use crate::compilation::{Compilation, Source};
use crate::config::Config;

/// A single pipeline stage.
///
/// Passes are stateless — all state lives in [`Compilation`].
/// Config provides the pluggable components (resolver, interpreter, backend).
pub trait Pass {
    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), String>;

    /// Whether this pass should run given the current compilation state.
    ///
    /// Default implementation always returns `true`. Override to skip
    /// passes when their preconditions are not met (e.g., skip
    /// `TypecheckPass` when no `expr` or `arg` keys exist).
    fn should_run(&self, _compilation: &Compilation, _config: &Config) -> bool {
        true
    }
}

/// A configured pipeline — holds config + ordered passes.
pub struct Pipeline {
    config: Config,
    pub(crate) passes: Vec<Box<dyn Pass>>,
}

impl Pipeline {
    pub fn new(config: Config) -> Self {
        Pipeline {
            config,
            passes: Vec::new(),
        }
    }

    /// Add a pass to the end of the pipeline.
    pub fn with_pass<P: Pass + 'static>(mut self, pass: P) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// Run all passes in order, skipping any whose `should_run` returns false.
    pub fn run(&self, source: Source) -> Result<Compilation, String> {
        let mut compilation = Compilation::new(source);
        for pass in &self.passes {
            if pass.should_run(&compilation, &self.config) {
                pass.run(&mut compilation, &self.config)?;
            }
        }
        compilation.resolve_error_locations();
        Ok(compilation)
    }
}
