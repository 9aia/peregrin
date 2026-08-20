//! # ac-runtime
//!
//! The concrete runtime value representation.
//!
//! `RuntimeValue` is a transparent newtype around `serde_json::Value`.
//! This crate is the only place that couples to `serde_json` —
//! `ac-core` stays dependency-free by being generic over the value type.
//!
//! JSON Schema gives these values meaning ("this is a User",
//! "this is a String", etc.). The VM evaluates expressions that
//! produce `RuntimeValue`s without knowing what the values mean.

mod value;
mod frontend;

pub use value::{HostValue, RuntimeValue};
pub use frontend::Frontend;

#[cfg(test)]
mod value_tests;
