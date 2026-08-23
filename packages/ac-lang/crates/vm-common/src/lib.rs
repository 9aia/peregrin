//! # ac-vm-common
//!
//! Reusable building blocks, mocks, and helpers for `ac-vm`.
//!
//! This crate is the "Lego library" for the VM: host combinators,
//! function builders, test mocks, and sandbox utilities. It depends
//! on `ac-vm` and only contains generic machinery — no concrete
//! evaluator and no built-in function semantics.

pub mod builder;
pub mod combinator;
pub mod mock;
pub mod prelude;
pub mod sandbox;

pub use builder::{
    fn_async, fn_function, fn_generic, fn_lazy, fn_operation, FnAsync, FnFunction, FnGeneric,
    FnLazy, FnOperation,
};
pub use combinator::{ChainHost, FilterHost, SpyHost};
pub use mock::{CaptureFn, MockHost, NoopHost};
pub use sandbox::{with_limits, with_max_calls, with_max_depth};
