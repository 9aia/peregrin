//! # ac-module-resolver-virtual
//!
//! In-memory [`ModuleResolver`] implementation.
//!
//! Stores modules in a `HashMap<String, JsonValue>` — no filesystem I/O.
//! Ideal for tests, programmatic usage, and embedded scenarios.

mod virtual_resolver;

pub use virtual_resolver::VirtualResolver;

#[cfg(test)]
mod lib_tests;
