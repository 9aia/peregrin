//! # ac-runtime
//!
//! The concrete runtime value representation.
//!
//! `NonFunctionValue` is a transparent newtype around `serde_json::Value`.
//! This crate is the only place that couples to `serde_json` —
//! `ac-core` stays dependency-free by being generic over the value type.
//!
//! JSON Schema gives these values meaning ("this is a User",
//! "this is a String", etc.). The VM evaluates expressions that
//! produce `NonFunctionValue`s without knowing what the values mean.

mod frontend;
mod value;

pub use frontend::Frontend;
pub use value::{FunctionValue, HostValue, NonFunctionValue};

#[cfg(test)]
mod value_tests;
