//! # ac-lib-compress
//!
//! Compression abstractions for the AC runtime.
//!
//! Defines format-agnostic [`Compressor`], [`Decompressor`], and [`Codec`]
//! traits so individual format crates can be wired in as separate
//! dependencies and benchmarked independently.

pub mod error;
pub mod traits;

pub use error::CompressError;
pub use traits::{Codec, Compressor, Decompressor};
