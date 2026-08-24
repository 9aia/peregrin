//! # ac-module-resolver-noop
//!
//! No-op [`ModuleCache`] implementation — every `get` returns `None`.
//!
//! Use when caching is not desired (e.g., tests that need fresh I/O
//! on every resolve, or memory-constrained environments).

mod no_cache;

pub use no_cache::NoCache;

#[cfg(test)]
mod lib_tests;