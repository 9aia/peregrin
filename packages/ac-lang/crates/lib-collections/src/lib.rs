//! # ac-lib-collections
//!
//! Collection types for the AC runtime.
//!
//! Provides the `collections` module with `vec`, `list`, `queue`,
//! `stack`, `deque`, `hash_map`, `hash_set`, `btree_map`, `btree_set`,
//! `priority_queue`, and `bit_set`.
//!
//! Currently implemented:
//!
//! - `vec.*` — back-ended dynamic array
//! - `list.*` — front-ended list
//! - `queue.*` — FIFO queue
//! - `stack.*` — LIFO stack
//! - `deque.*` — double-ended queue
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_modules::ModuleHost;
//! use ac_lib_collections::CollectionsModule;
//!
//! let host = ModuleHost::new()
//!     .with(CollectionsModule);
//! ```

mod bit_set;
mod btree_map;
mod btree_set;
mod common;
mod deque;
mod error;
mod hash_map;
mod hash_set;
mod list;
mod module;
mod ops;
pub mod priority_queue;
pub mod queue;
pub mod stack;
pub mod vec;

pub use error::CollectionsError;
pub use module::CollectionsModule;
