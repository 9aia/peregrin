//! Convenience prelude for VM consumers and tests.

pub use ac_runtime::{
    AsyncEval, AsyncFunction, Context, Error, Eval, Expr, Function, GenericFunction, HandlerClause,
    Host, HostRef, LazyFunction, Operation, OperationSig, Value, VirtualMachine,
};
pub use ac_runtime_values::{CanonicalContext, FunctionValue, NonFunctionValue};

pub use crate::{
    AllowHost, CaptureFn, ChainHost, Const, ContextBuilder, DenyHost, FilterHost, FnAsync,
    FnFunction, FnGeneric, FnLazy, FnOperation, HostEvent, HostEventKind, Identity, If, Logged,
    LoggingHost, Machine, MapArgs, MapHost, MapResult, Memoize, MockHost, NoopHost, Partial, Pipe,
    SpyHost, Timed, catch, catch_operation, r#const, fn_async, fn_function, fn_generic, fn_lazy,
    fn_operation, identity, r#if, logged, map_args, map_result, memoize, partial, pipe, timed,
    with_limits, with_max_calls, with_max_depth,
};
