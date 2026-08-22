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
mod continuation;
mod error;
mod eval;
mod host;
mod operation;
mod vm;

pub use context::Context;
pub use continuation::{Continuation, EvalResult, Frame};
pub use error::Error;
pub use eval::{AsyncEval, Eval};
pub use host::{AsyncFunction, Function, GenericFunction, Host, LazyFunction};
pub use operation::{Operation, OperationSig};
pub use vm::Vm;

#[cfg(test)]
mod context_tests;

#[cfg(test)]
mod continuation_tests;

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod generic_tests;

#[cfg(test)]
mod handler_tests;

#[cfg(test)]
mod operation_tests;

#[cfg(test)]
mod vm_tests;
