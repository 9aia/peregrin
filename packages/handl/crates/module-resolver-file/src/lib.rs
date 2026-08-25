//! # handl-module-resolver-file
//!
//! Filesystem-based [`ModuleResolver`] implementation.
//!
//! Reads files from disk, parses them with the appropriate frontend
//! (YAML/JSON/TOML based on file extension), and extracts fields.
//! Supports optional [`ModuleCache`] for memoizing resolved modules.

mod file_resolver;

pub use file_resolver::FileResolver;

#[cfg(test)]
mod lib_tests;
