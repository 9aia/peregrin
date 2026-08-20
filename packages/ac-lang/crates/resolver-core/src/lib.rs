//! # ac-resolver-core
//!
//! Module resolution abstraction — decoupled from typegen.
//!
//! Defines the [`ModuleResolver`] trait and [`ResolvedModule`] struct.
//! Implementations live in separate crates:
//!
//! - `ac-resolver-file` — filesystem-based (`std::fs`)
//! - `ac-resolver-virtual` — in-memory (`HashMap`)
//! - `ac-resolver-http` — HTTP(S) via `ureq`
//! - `ac-resolver-git` — Git repositories via `git2`
//!
//! Any system that needs to follow `import()` calls (typegen, validation,
//! linting) can take `&dyn ModuleResolver` without caring where modules
//! come from.

mod resolver;
mod cache;
mod caching;
mod chain;

pub use resolver::{ModuleResolver, ResolvedModule};
pub use cache::{ModuleCache, MemoryModuleCache, NoCache};
pub use caching::CachingResolver;
pub use chain::ChainResolver;

/// The conventional name of the host function that triggers module resolution.
///
/// This is a runtime host convention, not a language-level requirement.
/// The language defines no built-in functions — `import` is simply the name
/// the reference runtime uses for the function that loads external documents.
/// A runtime host may use a different name or provide module loading through
/// a different mechanism entirely.
pub const IMPORT_FUNCTION: &str = "import";
