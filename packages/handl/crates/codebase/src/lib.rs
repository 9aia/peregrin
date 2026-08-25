//! # handl-codebase
//!
//! Content-addressable document store for handl language codebases.
//!
//! A document is a collection of named definitions (functions, handlers).
//! Each document is identified by the hash of its canonical JSON
//! representation. Documents can import definitions from other documents
//! by hash, enabling reproducible builds and cross-machine sharing.
//!
//! ## Core types
//!
//! - [`DocumentHash`] — content hash of a document (SHA-256, hex-encoded).
//! - [`Document`] — a named collection of definitions.
//! - [`DocumentStore`] — stores documents by hash, with dependency tracking.

mod document;
mod store;

pub use document::{Document, DocumentEntry};
pub use store::DocumentStore;

#[cfg(test)]
mod tests;
