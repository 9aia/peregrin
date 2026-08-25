//! # handl-vm-host-canonical
//!
//! Canonical host implementation for the handl VM.
//!
//! Provides built-in functions (`var`, `resolve`, and `apply`) that are
//! available in all handl expressions. This crate is interface-agnostic
//! — usable from CLI, MCP, or any other consumer that needs to evaluate
//! handl expressions.

mod functions;
mod host;

pub use functions::{Apply, Resolve, Var};
pub use host::CanonicalHost;
