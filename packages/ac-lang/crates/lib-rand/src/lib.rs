//! # ac-lib-rand
//!
//! Random number generation and sampling library for the AC runtime.
//!
//! Provides functions for generating random integers, floats, booleans,
//! picking and shuffling elements, generating random strings and byte
//! arrays, and producing version-4 UUIDs.
//!
//! ## Architecture
//!
//! ```text
//! ac-runtime-core          ac-lib-rand
//! ─────────────            ────────────
//! Module trait             RandModule (Copy, zero-alloc)
//! ModuleHost               *Function structs
//!                          RandError → VmError
//! ```
//!
//! All function structs are `Copy` with no internal state. Each call
//! draws from the system RNG independently.
//!
//! ## Functions
//!
//! - [`RandModule`] — `rand.int`, `rand.float`, `rand.bool`,
//!   `rand.pick`, `rand.sample`, `rand.shuffle`, `rand.weighted_pick`,
//!   `rand.string`, `rand.bytes`, `rand.uuid`, `rand.gauss`,
//!   `rand.perlin_noise`
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_rand::RandModule;
//!
//! let host = ModuleHost::new()
//!     .with(RandModule);
//! ```
//!
//! Expressions can then call `rand.int(1, 6)`, `rand.float()`,
//! `rand.bool()`, `rand.pick(["a", "b", "c"])`, `rand.sample([1,2,3], 2)`,
//! `rand.shuffle([1,2,3])`, `rand.weighted_pick(["a","b"], [1,3])`,
//! `rand.string(16)`, `rand.bytes(8)`, `rand.uuid()`, etc.

mod common;
mod error;
mod module;
mod random;

pub use error::RandError;
pub use module::RandModule;

#[cfg(test)]
mod random_tests;
