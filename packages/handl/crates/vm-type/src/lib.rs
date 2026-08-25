//! # handl-vm-type
//!
//! Proof-of-concept: type checking with types represented as JSON values
//! and `is_compatible` implemented as an handl function (a
//! `FunctionValue` with a body expression string) evaluated by the VM.
//!
//! This demonstrates the migration of the handl type system from Rust enums
//! to handl values: types are encoded as JSON (via `Type::to_json`),
//! and compatibility checking is performed by an handl function body
//! that uses VM primitives (`bool.if`, `str.equals`, `obj.get`, etc.)
//! rather than Rust pattern matching.

pub mod compatibility;
pub mod eval;
pub mod host;

pub use compatibility::{
    eval_is_compatible_handl, is_compatible_function_value, is_compatible_json,
};
pub use eval::TypeEval;
pub use host::{
    AppArgsCompatible, FuncParamsCompatible, ObjEvery, PairSecond, TypeHelpersModule, TypeHost,
    TypeKind, TypeName, ZipAll,
};

#[cfg(test)]
mod tests;
