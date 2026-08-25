//! # handl-lib-utils-bytes
//!
//! Byte-array utility library for the handl runtime.
//!
//! Provides functions for inspecting, transforming, encoding and decoding
//! byte data.
//!
//! ## Functions
//!
//! - [`BytesModule`] — `bytes.length`, `bytes.is_empty`, `bytes.get`,
//!   `bytes.set`, `bytes.slice`, `bytes.concat`, `bytes.reverse`,
//!   `bytes.index_of`, `bytes.find`, `bytes.contains`, `bytes.starts_with`,
//!   `bytes.ends_with`, `bytes.equals`, `bytes.count`, `bytes.sum`,
//!   `bytes.min`, `bytes.max`, `bytes.avg`, `bytes.sort`, `bytes.from_string`,
//!   `bytes.to_string`, `bytes.encode_hex`, `bytes.decode_hex`,
//!   `bytes.encode_base64`, `bytes.decode_base64`
//!
//! ## Usage
//!
//! ```ignore
//! use handl_vm_common::ModuleHost;
//! use handl_lib_utils_bytes::BytesModule;
//!
//! let host = ModuleHost::new().with(BytesModule);
//! ```
//!
//! This crate is interface-agnostic — usable from CLI, MCP, or any consumer
//! that evaluates handl expressions.

mod common;
mod module;

pub use module::BytesModule;

#[cfg(test)]
mod module_tests;
