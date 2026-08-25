//! # handl-store
//!
//! Content-addressable definition store for the handl language.
//!
//! Definitions (functions, handlers) are identified by the hash of
//! their canonical JSON representation. Names are aliases that
//! resolve to hashes at compile time. The VM sees hash-encoded
//! strings — no AST change needed.
//!
//! ## Core types
//!
//! - [`DefinitionHash`] — a content hash (SHA-256, hex-encoded).
//! - [`DefinitionStore`] — stores [`FunctionValue`]s by hash.
//! - [`NameRegistry`] — maps names to [`DefinitionHash`]es.
//!
//! ## Canonicalization
//!
//! Hashing is deterministic: the same definition always produces the
//! same hash. Canonical JSON is produced by sorting object keys and
//! serializing with `serde_json` in canonical form.

mod canonical;
mod eval_cache;
mod handler;
mod hash;
mod store;

pub use canonical::canonicalize_json;
pub use eval_cache::{EvalCache, EvalKey};
pub use handler::{handler_clause_to_json, hash_expr, hash_handler_clause};
pub use hash::DefinitionHash;
pub use store::{DefinitionStore, NameRegistry};

#[cfg(test)]
mod tests;
