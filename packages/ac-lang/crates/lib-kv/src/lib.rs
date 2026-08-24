//! # ac-lib-kv
//!
//! Key-value store abstraction for the AC runtime.
//!
//! Defines the [`KeyValueStore`] trait, [`KeyValueStoreModule`] adapter, and
//! [`KvError`]. The `kv.*` functions are callable from AC expressions:
//! `kv.get("key")`, `kv.set("key", "value")`, etc.
//!
//! Unlike platform services (filesystem, env, http), a key-value store is a
//! stateful data abstraction — not tied to any OS capability. Backends may
//! include in-memory HashMaps, sled, SQLite, Redis, etc.
//!
//! - [`ac_lib_kv_hashmap`] — in-memory `HashMap` backend
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_modules::ModuleHost;
//! use ac_lib_kv::KeyValueStoreModule;
//! use ac_lib_kv_hashmap::HashKeyValueStore;
//!
//! let host = ModuleHost::new()
//!     .with(KeyValueStoreModule::new(HashKeyValueStore::default()));
//! ```

#![allow(clippy::module_inception)]

mod error;
mod key_value_store;

pub use error::KvError;
pub use key_value_store::{KeyValueStore, KeyValueStoreModule};
