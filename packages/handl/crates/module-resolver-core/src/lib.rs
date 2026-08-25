//! # handl-module-resolver-core
//!
//! Module resolution abstraction.
//!
//! Defines the [`ModuleResolver`] trait and [`ResolvedModule`] struct.
//! Implementations live in separate crates:
//!
//! - `handl-module-resolver-file` — filesystem-based (`std::fs`)
//! - `handl-module-resolver-virtual` — in-memory (`HashMap`)
//! - `handl-module-resolver-http` — HTTP(S) via `ureq`
//! - `handl-module-resolver-git` — Git repositories via `git2`
//! - `handl-module-resolver-memory` — in-memory `ModuleCache` implementation
//! - `handl-module-resolver-noop` — no-op `ModuleCache` implementation
//!
//! Any system that needs to follow `import()` calls (typegen, validation,
//! linting) can take `&dyn ModuleResolver` without caring where modules
//! come from.

mod module_cache;
mod module_caching_resolver;
mod module_chain_resolver;
mod module_resolver;
mod resolved_module;

pub use module_cache::ModuleCache;
pub use module_caching_resolver::CachingResolver;
pub use module_chain_resolver::ChainResolver;
pub use module_resolver::ModuleResolver;
pub use resolved_module::ResolvedModule;

/// The conventional name of the host function that triggers module resolution.
///
/// This is a runtime host convention, not a language-level requirement.
/// The language defines no built-in functions — `import` is simply the name
/// the reference runtime uses for the function that loads external documents.
/// A runtime host may use a different name or provide module loading through
/// a different mechanism entirely.
pub const IMPORT_FUNCTION: &str = "import";

pub mod resolved_module_test_helpers;

#[cfg(test)]
mod stubs {
    pub mod counting_resolver_stub;
    pub mod module_cache_stub;
    pub mod none_resolver_stub;
    pub mod static_resolver_stub;
}
#[cfg(test)]
mod module_caching_resolver_tests;
#[cfg(test)]
mod module_chain_resolver_tests;
#[cfg(test)]
mod module_resolver_tests;
