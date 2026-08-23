//! Optional host capabilities.
//!
//! Each capability is behind a Cargo feature so `ac-runtime-common`
//! stays cheap by default.

#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "module")]
pub mod module;
