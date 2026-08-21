//! # ac-runtime-core
//!
//! Abstract module system for the AC runtime.
//!
//! The AC language specification defines function calls as `fn(args)`. This
//! crate abstracts how functions are organized into named **modules** and how
//! a composite host dispatches qualified names like `math.abs(x)`.
//!
//! ## Architectural rule
//!
//! > **The VM knows nothing about modules.** The `Module` trait groups
//! > functions by namespace; `ModuleHost` composes modules into a single
//! > `Host` implementation. The VM sees only `Host::function(name)`.
//!
//! This crate defines the interface only. Concrete module implementations
//! live in `ac-runtime-std`.

#![allow(clippy::module_inception)]

mod host;
mod module;

pub use host::ModuleHost;
pub use module::Module;
