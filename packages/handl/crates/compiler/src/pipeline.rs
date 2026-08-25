//! Pass trait and Pipeline builder.
//!
//! The pipeline is a sequence of [`Pass`]es that run in order, each
//! receiving `&mut Compilation` and `&Config`. Passes are the extension
//! point — custom passes (linting, optimization, JIT) can be added
//! without modifying the core.
//!
//! Built-in pass implementations live in separate crates:
//!
//! - `handl-compiler-pass-parse` — `ParsePass`
//! - `handl-compiler-pass-extract` — `ExtractPass`
//! - `handl-compiler-pass-resolve` — `ResolvePass`
//! - `handl-compiler-pass-typecheck` — `TypecheckPass`
//! - `handl-compiler-pass-generate` — `GeneratePass`
//!
//! Presets (typegen, typecheck, full) are provided by `handl-compiler`,
//! which depends on all pass crates. This crate stays pass-agnostic.
//!
//! ## Custom passes
//!
//! ```ignore
//! struct LintPass;
//! impl Pass for LintPass {
//!     fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), PassError> {
//!         // read compilation.extracted, emit diagnostics
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
//!
//! ## Pass groups
//!
//! ```ignore
//! let pipeline = Pipeline::new(config)
//!     .with_pass(ParsePass)
//!     .with_pass(ExtractPass)
//!     .with_pass(PassGroup::new("optimization", vec![
//!         Box::new(DeadFieldElimPass),
//!         Box::new(ConstantFoldPass),
//!     ]))
//!     .with_pass(GeneratePass);
//! ```

use std::time::Instant;

use crate::compilation::{Compilation, Source};
use crate::config::Config;
use crate::error::PassError;
use crate::pass_meta::{PassCategory, PassMetadata};

/// A single pipeline stage.
///
/// Passes are stateless — all state lives in [`Compilation`].
/// Config provides the pluggable components (resolver, interpreter, backend).
pub trait Pass {
    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), PassError>;

    /// Whether this pass should run given the current compilation state.
    ///
    /// Default implementation always returns `true`. Override to skip
    /// passes when their preconditions are not met (e.g., skip
    /// `TypecheckPass` when no `expr` or `arg` keys exist).
    fn should_run(&self, _compilation: &Compilation, _config: &Config) -> bool {
        true
    }

    /// Metadata for diagnostics, ordering validation, and tooling.
    fn metadata(&self) -> PassMetadata {
        PassMetadata::default()
    }
}

/// A group of passes that runs as a single pass.
///
/// Enables composition: "optimization phase" = 3 passes, used as one unit.
/// Respects `stop_on_error` — if a sub-pass emits error diagnostics
/// (via `Compilation::emit`) and the group is configured to stop, later
/// sub-passes are skipped.
pub struct PassGroup {
    passes: Vec<Box<dyn Pass>>,
    metadata: PassMetadata,
    stop_on_error: bool}

impl PassGroup {
    pub fn new(name: &'static str, passes: Vec<Box<dyn Pass>>) -> Self {
        PassGroup {
            passes,
            metadata: PassMetadata::new(name),
            stop_on_error: true}
    }

    pub fn with_category(mut self, category: PassCategory) -> Self {
        self.metadata = self.metadata.with_category(category);
        self
    }

    /// Set whether this group stops on the first error diagnostic.
    /// Default: `true`.
    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }
}

impl Pass for PassGroup {
    fn run(&self, compilation: &mut Compilation, config: &Config) -> Result<(), PassError> {
        for pass in &self.passes {
            if pass.should_run(compilation, config) {
                pass.run(compilation, config)?;
                if self.stop_on_error && compilation.has_errors() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn should_run(&self, compilation: &Compilation, config: &Config) -> bool {
        self.passes
            .iter()
            .any(|p| p.should_run(compilation, config))
    }

    fn metadata(&self) -> PassMetadata {
        self.metadata.clone()
    }
}

/// A configured pipeline — holds config + ordered passes.
pub struct Pipeline {
    config: Config,
    pub(crate) passes: Vec<Box<dyn Pass>>,
    stop_on_error: bool,
    auto_validate: bool,
    instrumented: bool}

impl Pipeline {
    pub fn new(config: Config) -> Self {
        Pipeline {
            config,
            passes: Vec::new(),
            stop_on_error: true,
            auto_validate: true,
            instrumented: false}
    }

    /// Add a pass to the end of the pipeline.
    pub fn with_pass<P: Pass + 'static>(mut self, pass: P) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// Insert a pass before the first existing pass whose metadata
    /// name matches `target`. Returns an error if `target` is not found.
    pub fn with_pass_before<P: Pass + 'static>(
        mut self,
        target: &str,
        pass: P,
    ) -> Result<Self, PassError> {
        let pos = self
            .passes
            .iter()
            .position(|p| p.metadata().name == target)
            .ok_or_else(|| {
                PassError::new(
                    "pipeline",
                    format!(
                        "with_pass_before: target pass '{}' not found in pipeline",
                        target
                    ),
                )
            })?;
        self.passes.insert(pos, Box::new(pass));
        Ok(self)
    }

    /// Insert a pass after the first existing pass whose metadata
    /// name matches `target`. Returns an error if `target` is not found.
    pub fn with_pass_after<P: Pass + 'static>(
        mut self,
        target: &str,
        pass: P,
    ) -> Result<Self, PassError> {
        let pos = self
            .passes
            .iter()
            .position(|p| p.metadata().name == target)
            .ok_or_else(|| {
                PassError::new(
                    "pipeline",
                    format!(
                        "with_pass_after: target pass '{}' not found in pipeline",
                        target
                    ),
                )
            })?;
        self.passes.insert(pos + 1, Box::new(pass));
        Ok(self)
    }

    /// Flatten another pipeline's passes into this one.
    pub fn with_pipeline(mut self, other: Pipeline) -> Self {
        self.passes.extend(other.passes);
        self
    }

    /// Set whether the pipeline stops on the first error or collects
    /// all diagnostics. Default: `true` (stop on error).
    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// Enable or disable automatic pass ordering validation before
    /// `run()`. Default: `true`. When enabled, `run()` returns an error
    /// if pass ordering is invalid.
    pub fn auto_validate(mut self, validate: bool) -> Self {
        self.auto_validate = validate;
        self
    }

    /// Enable per-pass timing instrumentation. When enabled, each pass's
    /// duration is logged via `eprintln!`.
    pub fn instrumented(mut self, on: bool) -> Self {
        self.instrumented = on;
        self
    }

    /// Validate pass ordering — checks that every consumed IR stage
    /// is produced by an earlier pass.
    pub fn validate(&self) -> Result<(), PassError> {
        let mut produced: Vec<&str> = Vec::new();
        for pass in &self.passes {
            let meta = pass.metadata();
            for consumed in meta.consumes {
                if !produced.contains(consumed) {
                    return Err(PassError::new(
                        "pipeline",
                        format!(
                            "pass '{}' consumes '{}' which is not produced by an earlier pass",
                            meta.name, consumed
                        ),
                    ));
                }
            }
            for produces in meta.produces {
                produced.push(produces);
            }
        }
        Ok(())
    }

    /// Run all passes in order, skipping any whose `should_run` returns false.
    pub fn run(&self, source: Source) -> Result<Compilation, PassError> {
        if self.auto_validate {
            self.validate()?;
        }

        let mut compilation = Compilation::new(source);
        for pass in &self.passes {
            if pass.should_run(&compilation, &self.config) {
                let name = pass.metadata().name;
                let start = if self.instrumented {
                    Some(Instant::now())
                } else {
                    None
                };

                pass.run(&mut compilation, &self.config)?;

                if let Some(start) = start {
                    let elapsed = start.elapsed();
                    if elapsed.as_millis() > 0 {
                        eprintln!("[pipeline] pass '{}' took {:?}", name, elapsed);
                    }
                }

                if self.stop_on_error && compilation.has_errors() {
                    break;
                }
            }
        }
        compilation.resolve_error_locations();
        Ok(compilation)
    }
}