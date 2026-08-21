//! # ac-vm
//!
//! The execution engine for the AC language.
//!
//! This crate evaluates [`ac_ast::Expr`] trees using a
//! [`Vm`] driven by a user-provided [`Host`]. The VM has no
//! built-in functions — all capabilities are provided by the host
//! through the [`Primitive`] trait.
//!
//! ## Architectural rule
//!
//! > **`core` describes things. `vm` executes things.**
//!
//! The VM depends on `ac-ast` and nothing else. It does not know
//! about JSON, YAML, or JSON Schema. It does not know what `now()`
//! or `env()` means — those are host primitives.

mod context;
mod error;
mod eval;
mod host;
mod vm;

pub use context::Context;
pub use error::Error;
pub use eval::Eval;
pub use host::{Function, Host};
pub use vm::Vm;

#[cfg(test)]
mod context_tests;

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod vm_tests;
