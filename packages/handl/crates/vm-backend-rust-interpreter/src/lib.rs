//! # handl-vm-backend-rust-interpreter
//!
//! Canonical tree-walking virtual machine for the handl language.
//!
//! Implements `handl_vm::VirtualMachine` using the canonical handl expression
//! tree and host-function dispatch. The core contracts are defined in
//! `handl-vm` and re-exported here for convenience.

pub use handl_vm::{
    AsyncEval, AsyncFunction, Context, Error, Eval, Expr, Function, GenericFunction, HandlerClause,
    Host, HostRef, LazyFunction, Operation, OperationSig, Pattern, Value, VirtualMachine,
};
pub use handl_vm_values_json::{CanonicalContext, FunctionValue, NonFunctionValue};

mod continuation;
mod eval;
mod handlers;
mod pattern;
mod vm;

#[cfg(test)]
mod continuation_tests;
#[cfg(test)]
mod generic_tests;
#[cfg(test)]
mod handler_tests;
#[cfg(test)]
mod operation_tests;
#[cfg(test)]
mod vm_tests;

pub use continuation::{Continuation, EvalResult, Frame};
pub use eval::CanonicalEval;
pub use handlers::{
    ArrayHandler, CallHandler, HandleHandler, ObjectHandler, ValueHandler, canonical_vm,
};
pub use vm::CanonicalVm;
pub use vm::CanonicalVm as Vm;
