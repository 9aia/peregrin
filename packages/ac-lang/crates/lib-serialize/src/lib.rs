//! # ac-lib-serialize
//!
//! Format-agnostic serialization framework.
//!
//! Provides `Encoder`, `Decoder`, `serialize`, and `deserialize` abstractions
//! shared by the `ac-lib-*` format crates.

pub mod error;
pub mod traits;

pub use error::SerializeError;
pub use traits::{Decoder, Encoder};

/// Serialize a value using the provided encoder.
pub fn serialize<T, E>(value: &T, encoder: &E) -> Result<E::Output, SerializeError>
where
    E: Encoder<T>,
{
    encoder.encode(value)
}

/// Deserialize input using the provided decoder.
pub fn deserialize<I, T, D>(input: &I, decoder: &D) -> Result<T, SerializeError>
where
    D: Decoder<I, T>,
{
    decoder.decode(input)
}
