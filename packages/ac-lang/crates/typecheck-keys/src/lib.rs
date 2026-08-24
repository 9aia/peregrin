//! # ac-typecheck-keys
//!
//! Typechecks individual AC keys (`arg`, `expr`, `merge`,
//! `inheritable`, `mutable`, `termination`).
//!
//! Each key is implemented as a module under this crate.

pub mod arg;
pub mod expr;
pub mod inherit;
pub mod merge;
pub mod mutable;
pub mod termination;
