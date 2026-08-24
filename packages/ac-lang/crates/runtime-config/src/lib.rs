//! # ac-runtime-config
//!
//! The configuration-language value representation for AC.
//!
//! `NonFunctionValue` is a transparent newtype around `serde_json::Value`,
//! and `FunctionValue` is the JSON-backed, closure-free first-class function
//! encoding. These are the concrete value types for YAML/JSON/TOML-style
//! configuration documents. They are not the universal default for all
//! AC-Lang runtimes — the core VM is generic over the value type.

mod canonical_context;
mod value;

#[cfg(test)]
mod canonical_context_tests;
#[cfg(test)]
mod error_tests;

pub use canonical_context::CanonicalContext;
pub use value::{FunctionValue, HostValue, NonFunctionValue};

/// Concrete error type for the JSON-backed runtime configuration.
pub type Error = ac_runtime::Error<NonFunctionValue>;
