//! # ac-meta
//!
//! Meta host functions for the AC language runtime.
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
//! Use [`MetaHost`] as a drop-in replacement for [`DefaultHost`] when
//! you want meta functions available in your AC expressions:
//!
//! ```ignore
//! use ac_meta::MetaHost;
//! use ac_runtime_canonical::{CanonicalContext, Vm, Context};
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
