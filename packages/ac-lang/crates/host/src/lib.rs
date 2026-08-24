//! # ac-host
//!
//! Default host implementation for the AC VM.
//!
//! Provides built-in functions (`var`, `resolve`, and `apply`) that are
//! available in all AC expressions. This crate is interface-agnostic
//! — usable from CLI, MCP, or any other consumer that needs to evaluate
//! AC expressions.

mod functions;
mod host;

pub use functions::{Apply, Resolve, Var};
pub use host::DefaultHost;
