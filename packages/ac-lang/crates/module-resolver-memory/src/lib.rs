//! # ac-module-resolver-memory
//!
//! In-memory [`ModuleCache`] implementation backed by a `Mutex<HashMap>`.
//!
//! Thread-safe — suitable for both single-threaded and multi-threaded
//! pipeline usage (e.g. parallel type checking with rayon).

mod memory_module_cache;

pub use memory_module_cache::MemoryModuleCache;

#[cfg(test)]
mod lib_tests;