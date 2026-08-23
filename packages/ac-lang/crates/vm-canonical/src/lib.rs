//! # ac-vm-canonical
//!
//! Canonical tree-walking virtual machine for the AC language.
//!
//! Implements `ac_vm::VirtualMachine` using the canonical AC expression
//! tree and host-function dispatch. The core contracts are defined in
//! `ac-vm` and re-exported here for convenience.

pub use ac_vm::{
    AsyncEval, AsyncFunction, Context, Error, Eval, Expr, Function, GenericFunction,
    HandlerClause, Host, HostRef, LazyFunction, NonFunctionValue, Operation, OperationSig,
    RuntimeContext, VirtualMachine,
};

mod continuation;
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
pub use vm::CanonicalVm;
pub use vm::CanonicalVm as Vm;
