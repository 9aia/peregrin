//! Convenience prelude for VM consumers and tests.

pub use ac_vm::{AsyncEval, AsyncFunction, CanonicalContext, Context, Error, Eval, Expr, Function, GenericFunction, HandlerClause, Host, HostRef, LazyFunction, NonFunctionValue, Operation, OperationSig, VirtualMachine};

pub use crate::{
    catch, catch_operation, identity, logged, map_args, map_result, memoize, partial, pipe,
    r#const, r#if, timed, AllowHost, CaptureFn, ChainHost, Const, ContextBuilder, DenyHost, FnAsync,
    FnFunction, FnGeneric, FnLazy, FnOperation, HostEvent, HostEventKind, Identity, If, Logged,
    LoggingHost, Machine, MapArgs, MapHost, MapResult, Memoize, MockHost, NoopHost, Partial, Pipe,
    SpyHost, Timed,
    fn_async, fn_function, fn_generic, fn_lazy, fn_operation};
