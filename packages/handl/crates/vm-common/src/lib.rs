//! # handl-vm-common
//!
//! Reusable building blocks, mocks, and helpers for `handl-vm`.
//!
//! This crate is the "Lego library" for the VM: host combinators,
//! function builders, test mocks, and sandbox utilities. It depends
//! on `handl-vm` and only contains generic machinery — no concrete
//! evaluator and no built-in function semantics.

pub mod adapter;
pub mod builder;
pub mod capabilities;
pub mod combinator;
pub mod host;
pub mod lego;
pub mod machine;
pub mod middleware;
pub mod middleware_vm;
pub mod mock;
pub mod module;
pub mod prelude;
pub mod sandbox;

pub use host::ModuleHost;
pub use module::{Module, module_stub};

pub use adapter::{
    Catch, CatchOp, Const, Identity, If, Logged, MapArgs, MapResult, Memoize, Partial, Pipe, Timed,
    catch, catch_operation, r#const, identity, r#if, logged, map_args, map_result, memoize,
    partial, pipe, timed,
};
pub use builder::{
    FnAsync, FnFunction, FnGeneric, FnLazy, FnOperation, fn_async, fn_function, fn_generic,
    fn_lazy, fn_operation,
};
pub use combinator::{ChainHost, FilterHost, SpyHost};
pub use lego::{ExprHandler, LegoVm};
pub use machine::{ContextBuilder, Machine};
pub use middleware::{AllowHost, DenyHost, HostEvent, HostEventKind, LoggingHost, MapHost};
pub use middleware_vm::{
    Chain, Counter, EvalKind, MiddlewareVm, NoopMiddleware, Trace, TraceEvent, VmMiddleware,
};
pub use mock::{CaptureFn, MockHost, NoopHost};
pub use sandbox::{with_limits, with_max_calls, with_max_depth};
