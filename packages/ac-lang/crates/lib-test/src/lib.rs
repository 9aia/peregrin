//! # ac-lib-test
//!
//! First-class testing standard library for the AC runtime.
//!
//! Provides the runtime modules used by AC expressions:
//!
//! - [`TestModule`] — `test.run`, `test.parallel`, `test.fixture`
//! - [`AssertModule`] — `assert.equal`, `assert.not_equal`, `assert.true`,
//!   `assert.false`, `assert.contains`, `assert.throws`
//! - [`PropertyModule`] — `property.for_all`
//! - [`BenchmarkModule`] — `benchmark.run`
//! - [`FuzzModule`] — `fuzz.run`
//! - [`SnapshotModule`] — `snapshot.match`, `snapshot.format`
//!
//! All modules are stateless, `Copy`, and follow the `Function<NonFunctionValue>`
//! contract used by the rest of the standard library.

mod assert;
mod benchmark;
mod error;
mod fuzz;
mod property;
mod shared;
mod snapshot;
mod test;

pub use assert::AssertModule;
pub use benchmark::BenchmarkModule;
pub use error::TestError;
pub use fuzz::FuzzModule;
pub use property::PropertyModule;
pub use snapshot::SnapshotModule;
pub use test::TestModule;

#[cfg(test)]
mod assert_tests;
#[cfg(test)]
mod benchmark_tests;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
mod shared_tests;
#[cfg(test)]
mod test_tests;
