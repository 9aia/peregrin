//! # ac-host-resolver
//!
//! Module-resolver-backed host for the AC VM.
//!
//! Wraps any [`Host`](ac_vm::Host) and adds an `import()` function
//! that delegates to a [`ModuleResolver`](ac_module_resolver_core::ModuleResolver).
//! This enables programs to import external modules at runtime —
//! not just at type-check time.
//!
//! ## Architectural rule
//!
//! > **The host owns import; the VM owns nothing.**
//!
//! The VM has no built-in `import` function. This crate provides a
//! [`ResolverHost`] that wraps an existing host and intercepts
//! `import()` calls, delegating them to a `ModuleResolver`. The
//! resolver determines where modules come from (filesystem, HTTP,
//! git, in-memory, or a chain of these).
//!
//! ## Example
//!
//! ```ignore
//! use ac_host::DefaultHost;
//! use ac_host_resolver::ResolverHost;
//! use ac_module_resolver_virtual::VirtualResolver;
//! use ac_vm::{Vm, Context};
//! use ac_ast_canonical::Expr;
//!
//! let mut resolver = VirtualResolver::new();
//! resolver.add("config.yml", serde_json::json!({
//!     "$": { "host": { "type": "string" } },
//!     "host": "localhost"
//! }));
//!
//! let host = ResolverHost::new(
//!     DefaultHost::default(),
//!     Box::new(resolver),
//! );
//! let vm = Vm::new(host);
//! let mut ctx = Context::new();
//!
//! let expr = Expr::call("import", vec![Expr::value("config.yml")]);
//! let result = vm.eval(&expr, &mut ctx).unwrap();
//! ```

mod host;
mod import_function;

pub use host::ResolverHost;
pub use import_function::ImportFunction;
