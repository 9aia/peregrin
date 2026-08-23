//! Convenience prelude for VM consumers and tests.

pub use ac_vm::{
    AsyncEval, AsyncFunction, Context, Error, Eval, Expr, Function, GenericFunction, HandlerClause,
    Host, HostRef, LazyFunction, NonFunctionValue, Operation, OperationSig, RuntimeContext,
    VirtualMachine,
};

pub use crate::{
    fn_async, fn_function, fn_generic, fn_lazy, fn_operation, CaptureFn, ChainHost, FnAsync,
    FnFunction, FnGeneric, FnLazy, FnOperation, MockHost, NoopHost, SpyHost,
};
