//! # ac-lib-http-std
//!
//! Concrete HTTP client implementation backed by `ureq`.
//!
//! Provides [`StdHttpClient`] using the [`ureq`] crate.
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_modules::ModuleHost;
//! use ac_lib_http::HttpClientModule;
//! use ac_lib_http_std::StdHttpClient;
//!
//! let host = ModuleHost::new()
//!     .with(HttpClientModule::new(StdHttpClient));
//! ```

mod http_client;

pub use http_client::StdHttpClient;
