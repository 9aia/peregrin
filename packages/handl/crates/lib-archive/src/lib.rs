//! # handl-lib-archive
//!
//! Archive format abstractions for the handl runtime.
//!
//! Defines format-agnostic [`Archive`] trait and [`ArchiveEntry`] so
//! individual archive format crates can be wired in as separate
//! dependencies and benchmarked independently.

pub mod entry;
pub mod error;
pub mod traits;

pub use entry::ArchiveEntry;
pub use error::ArchiveError;
pub use traits::Archive;
