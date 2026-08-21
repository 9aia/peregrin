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
//! draws from the system RNG independently. Seeded variants use
//! `StdRng` seeded from a user-provided value for reproducibility.
//!
//! ## Functions
//!
//! - [`RandModule`] — `rand.int`, `rand.float`, `rand.bool`,
//!   `rand.pick`, `rand.choice`, `rand.sample`, `rand.shuffle`, `rand.weighted_pick`,
//!   `rand.string`, `rand.bytes`, `rand.uuid`, `rand.gauss`,
//!   `rand.exponential`, `rand.poisson`, `rand.binomial`,
//!   `rand.perlin_1d`, `rand.perlin_2d`, `rand.perlin_3d`,
//!   `rand.seeded_int`, `rand.seeded_float`, `rand.seeded_bool`,
//!   `rand.seeded_pick`, `rand.seeded_choice`, `rand.seeded_sample`,
//!   `rand.seeded_shuffle`, `rand.seeded_string`, `rand.seeded_gauss`
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
//! `rand.bool()`, `rand.pick(["a", "b", "c"])`, `rand.choice([1,2,3], 5)`, `rand.sample([1,2,3], 2)`,
//! `rand.shuffle([1,2,3])`, `rand.weighted_pick(["a","b"], [1,3])`,
//! `rand.string(16)`, `rand.bytes(8)`, `rand.uuid()`, `rand.gauss(0, 1)`,
//! `rand.perlin_1d(0.5)`, `rand.perlin_2d(1, 2)`, `rand.perlin_3d(0.1, 0.2, 0.3)`,
//! `rand.seeded_int(42, 1, 6)`, `rand.seeded_shuffle(123, [1,2,3])`, etc.

mod common;
mod distribution;
mod error;
mod generation;
mod module;
mod noise;
mod numbers;
mod sampling;
mod seeded;

pub use error::RandError;
pub use module::RandModule;

#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod numbers_tests;
#[cfg(test)]
mod sampling_tests;
#[cfg(test)]
mod generation_tests;
#[cfg(test)]
mod distribution_tests;
#[cfg(test)]
mod noise_tests;
#[cfg(test)]
mod seeded_tests;
#[cfg(test)]
mod module_tests;
