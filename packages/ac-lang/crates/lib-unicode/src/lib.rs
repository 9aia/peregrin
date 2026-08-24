//! # ac-lib-unicode
//!
//! Unicode primitives for the AC language toolchain and runtime.
//!
//! Provides a focused, compiler-friendly API over Unicode data:
//!
//! - [`decode`] — validate and decode UTF-8/UTF-16/UTF-32 byte streams into
//!   `char` iterators, with byte offsets for diagnostic positions.
//! - [`ident`] — identifier validation using `XID_Start` / `XID_Continue`
//!   (UAX #31), suitable for lexers and parsers.
//!
//! Future work: grapheme cluster iteration, normalization, and case folding.

pub mod decode;
pub mod ident;

pub use decode::{decode_utf8, DecodeError, Utf8Iterator};
pub use ident::{is_identifier_continue, is_identifier_start};
