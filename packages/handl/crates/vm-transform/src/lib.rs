//! # handl-vm-transform
//!
//! Generic AST / expression transformation utilities for the handl VM.
//!
//! This crate is intentionally minimal for now. It will grow into a
//! generic transformation layer as bytecode and JIT targets are added.

/// A placeholder no-op transformation that returns its input unchanged.
pub fn identity<T>(value: T) -> T {
    value
}
