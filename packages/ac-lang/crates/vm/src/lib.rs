//! # ac-vm
//!
//! Core virtual-machine abstractions for the AC language.
//!
//! This crate defines the evaluation contract: `VirtualMachine`,
//! `Context`, `Host`, `Function`, `Operation`, `Error`, etc.
//! It does not contain a concrete implementation; it is generic over
//! the value type `V` via the [`Value`] trait.
//!
//! The canonical tree-walking VM lives in `ac-vm-canonical`. Reusable
//! helpers, mocks, and host combinators live in `ac-vm-common`.

pub mod error;
pub mod eval;
pub mod function_repr;
pub mod host;
pub mod operation;
pub mod runtime;
pub mod value;

pub use ac_ast_canonical::{Expr, ExprFactory, HandlerClause, Pattern};
pub use error::Error;
pub use eval::{AsyncEval, Eval};
pub use function_repr::FunctionRepr;
pub use host::{AsyncFunction, Function, GenericFunction, Host, HostRef, LazyFunction};
pub use operation::{Operation, OperationSig};
pub use runtime::{Context, VirtualMachine};
pub use value::Value;
