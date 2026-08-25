//! # handl-vm-common::capabilities::module
//!
//! Module-resolver-backed host for the handl VM.
//!
//! Wraps any [`Host`](handl_vm::Host) and adds an `import()` function
//! that delegates to a [`ModuleResolver`](handl_module_resolver_core::ModuleResolver).
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
//! use handl_vm_host_canonical::CanonicalHost;
//! use handl_vm_common::capabilities::module::ResolverHost;
//! use handl_module_resolver_virtual::VirtualResolver;
//! use handl_vm_backend_rust_interpreter::{CanonicalContext, Vm, Context};
//! use handl_ast_canonical::Expr;
//!
//! let mut resolver = VirtualResolver::new();
//! resolver.add("config.yml", serde_json::json!({
//!     "$": { "host": { "type": "string" } },
//!     "host": "localhost"
//! }));
//!
//! let host = ResolverHost::new(
//!     CanonicalHost::default(),
//!     Box::new(resolver),
//! );
//! let vm = Vm::new(host);
//! let mut ctx = CanonicalContext::new();
//!
//! let expr = Expr::call("import", vec![Expr::value("config.yml")]);
//! let result = vm.eval(&expr, &mut ctx).unwrap();
//! ```

mod host;
mod import_function;

#[cfg(test)]
mod tests;

pub use host::ResolverHost;
pub use import_function::ImportFunction;
