//! # ac-vm
//!
//! Core virtual-machine abstractions for the AC language.
//!
//! This crate defines the evaluation contract: `VirtualMachine`,
//! `Context`, `Host`, `Function`, `Operation`, `Error`, etc.
//! It does not contain a concrete implementation.
//!
//! The canonical tree-walking VM lives in `ac-vm-canonical`. Reusable
//! helpers, mocks, and host combinators live in `ac-vm-common`.

pub mod context;
pub mod error;
pub mod eval;
pub mod host;
pub mod operation;
pub mod runtime;

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod error_tests;

pub use ac_ast_canonical::{Expr, HandlerClause};
pub use ac_runtime::NonFunctionValue;
pub use context::CanonicalContext;
pub use runtime::{Context, VirtualMachine};
pub use error::Error;
pub use eval::{AsyncEval, Eval};
pub use host::{
    AsyncFunction, Function, GenericFunction, Host, HostRef, LazyFunction};
pub use operation::{Operation, OperationSig};
