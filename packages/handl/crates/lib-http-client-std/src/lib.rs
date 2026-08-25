//! # handl-lib-http-client-std
//!
//! Concrete HTTP client implementation backed by `ureq`.
//!
//! Provides [`StdHttpClient`] using the [`ureq`] crate.
//!
//! ## Usage
//!
//! ```ignore
//! use handl_vm_common::ModuleHost;
//! use handl_lib_http_client::HttpClientModule;
//! use handl_lib_http_client_std::StdHttpClient;
//!
//! let host = ModuleHost::new()
//!     .with(HttpClientModule::new(StdHttpClient));
//! ```

mod http_client;

pub use http_client::StdHttpClient;
