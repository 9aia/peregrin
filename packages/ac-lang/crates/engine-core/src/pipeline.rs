//! Pass trait and Pipeline builder.
//!
//! The pipeline is a sequence of [`Pass`]es that run in order, each
//! receiving `&mut Compilation` and `&Config`. Passes are the extension
//! point — custom passes (linting, optimization, JIT) can be added
//! without modifying the core.
//!
//! Built-in pass implementations live in separate crates:
//!
//! - `ac-engine-pass-parse` — `ParsePass`
//! - `ac-engine-pass-extract` — `ExtractPass`
//! - `ac-engine-pass-resolve` — `ResolvePass`
//! - `ac-engine-pass-typecheck` — `TypecheckPass`
//! - `ac-engine-pass-generate` — `GeneratePass`
//!
//! Presets (typegen, typecheck, full) are provided by `ac-engine`,
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

    /// Run all passes in order.
    pub fn run(&self, source: Source) -> Result<Compilation, String> {
        let mut compilation = Compilation::new(source);
        for pass in &self.passes {
            pass.run(&mut compilation, &self.config)?;
        }
        Ok(compilation)
    }
}
