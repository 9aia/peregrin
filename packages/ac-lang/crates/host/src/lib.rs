//! # ac-host
//!
//! Default host implementation for the AC VM.
//!
//! Provides built-in functions (`add`, `mul`, `concat`, `len`) that
//! are available in all AC expressions. This crate is interface-agnostic
//! — usable from CLI, MCP, or any other consumer that needs to evaluate
//! AC expressions.

mod functions;
mod host;

pub use functions::{Add, Apply, Array, Concat, Len, Mul, Resolve, Var};
pub use host::DefaultHost;
