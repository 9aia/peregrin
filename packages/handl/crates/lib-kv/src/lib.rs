//! # handl-lib-kv
//!
//! Key-value store abstraction for the handl runtime.
//!
//! Defines the [`KeyValueStore`] trait, [`KeyValueStoreModule`] adapter, and
//! [`KvError`]. The `kv.*` functions are callable from handl expressions:
//! `kv.get("key")`, `kv.set("key", "value")`, etc.
//!
//! Unlike platform services (filesystem, env, http), a key-value store is a
//! stateful data abstraction — not tied to any OS capability. Backends may
//! include in-memory HashMaps, sled, SQLite, Redis, etc.
//!
//! - [`handl_lib_kv_hashmap`] — in-memory `HashMap` backend
//!
//! ## Usage
//!
//! ```ignore
//! use handl_vm_common::ModuleHost;
//! use handl_lib_kv::KeyValueStoreModule;
//! use handl_lib_kv_hashmap::HashKeyValueStore;
//!
//! let host = ModuleHost::new()
//!     .with(KeyValueStoreModule::new(HashKeyValueStore::default()));
//! ```

#![allow(clippy::module_inception)]

mod error;
mod key_value_store;

pub use error::KvError;
pub use key_value_store::{KeyValueStore, KeyValueStoreModule};
