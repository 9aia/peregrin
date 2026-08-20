//! # ac-core
//!
//! The language model — the kernel of the AC language.
//!
//! This crate describes things. It contains the representation of
//! expressions and fields. It is generic over the value type `V` —
//! the concrete value representation (`RuntimeValue`) lives in
//! `ac-runtime`, not here. This keeps the kernel dependency-free.
//!
//! ## Architectural rule
//!
//! > **`core` describes things. `vm` executes things. `schema`
//! > gives values meaning. `host` gives execution capabilities
//! > meaning. Formats translate external representations.**
//!
//! Keep this crate dependency-free and minimal. The type system
//! lives here, so changes to this crate ripple outward — the
//! smaller it is, the more stable the language.

mod expr;
mod field;

pub use expr::Expr;
pub use field::Field;

#[cfg(test)]
mod expr_test;

#[cfg(test)]
mod field_test;
