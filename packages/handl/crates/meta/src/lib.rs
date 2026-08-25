//! # handl-meta
//!
//! Meta host functions for the handl language runtime.
//!
//! Provides reflection (`typeof`, `keys`, `is_function`), dynamic
//! evaluation (`eval`), and AST manipulation (`parse`, `ast_to_string`,
//! `ast_name`, `ast_args`, `ast_is_call`) as host functions.
//!
//! These functions make the language self-reflective: expressions can
//! inspect values, construct and evaluate expression strings at runtime,
//! and manipulate the AST of expressions as data.
//!
//! ## Usage
//!
//! Use [`MetaHost`] as a drop-in replacement for [`CanonicalHost`] when
//! you want meta functions available in your handl expressions:
//!
//! ```ignore
//! use handl_meta::MetaHost;
//! use handl_vm_backend_rust_interpreter::{CanonicalContext, Vm, Context};
//!
//! let vm = Vm::new(MetaHost::default());
//! let mut ctx = CanonicalContext::new();
//! // eval("add(1, 2)") → 3
//! ```

mod functions;
mod host;

pub use functions::{
    AstArgs, AstIsCall, AstName, AstToString, Eval, IsFunction, Keys, Parse, TypeOf,
};
pub use host::MetaHost;

#[cfg(test)]
mod functions_tests;
