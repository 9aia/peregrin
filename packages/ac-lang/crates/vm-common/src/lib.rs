//! # ac-vm-common
//!
//! Reusable building blocks, mocks, and helpers for `ac-vm`.
//!
//! This crate is the "Lego library" for the VM: host combinators,
//! function builders, test mocks, and sandbox utilities. It depends
//! on `ac-vm` and only contains generic machinery — no concrete
//! evaluator and no built-in function semantics.

pub mod adapter;
pub mod builder;
pub mod capabilities;
pub mod combinator;
pub mod lego;
pub mod machine;
pub mod mock;
pub mod middleware;
pub mod prelude;
pub mod sandbox;
pub mod middleware_vm;

pub use adapter::{
    catch, catch_operation, identity, logged, map_args, map_result, memoize, partial, pipe,
    r#const, r#if, timed, Catch, CatchOp, Const, Identity, If, Logged, MapArgs, MapResult, Memoize,
    Partial, Pipe, Timed};
pub use builder::{
    fn_async, fn_function, fn_generic, fn_lazy, fn_operation, FnAsync, FnFunction, FnGeneric,
    FnLazy, FnOperation};
pub use combinator::{ChainHost, FilterHost, SpyHost};
pub use lego::{ExprHandler, LegoVm};
pub use machine::{ContextBuilder, Machine};
pub use mock::{CaptureFn, MockHost, NoopHost};
pub use middleware::{AllowHost, DenyHost, HostEvent, HostEventKind, LoggingHost, MapHost};
pub use sandbox::{with_limits, with_max_calls, with_max_depth};
pub use middleware_vm::{Chain, Counter, EvalKind, MiddlewareVm, NoopMiddleware, Trace, TraceEvent, VmMiddleware};
