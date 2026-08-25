//! # handl-lib-unicode
//!
//! Unicode primitives for the handl language toolchain and runtime.
//!
//! Provides a focused, compiler-friendly API over Unicode data:
//!
//! - [`decode`] — validate and decode UTF-8/UTF-16/UTF-32 byte streams into
//!   `char` iterators, with byte offsets for diagnostic positions.
//! - [`ident`] — identifier validation using `XID_Start` / `XID_Continue`
//!   (UAX #31), suitable for lexers and parsers.
//!
//! Also exposes the `unicode` handl runtime module via [`UnicodeModule`].
//!
//! Future work: grapheme cluster iteration, normalization, and case folding.

pub mod decode;
pub mod ident;
pub mod module;

pub use decode::{DecodeError, Utf8Iterator, decode_utf8};
pub use ident::{is_identifier, is_identifier_continue, is_identifier_start};
pub use module::UnicodeModule;
