//! # handl-lib-http-client
//!
//! Abstract HTTP client service trait for the handl runtime.
//!
//! Defines an abstract HTTP client interface that bridges HTTP request
//! capabilities into a callable [`Module`] implementation.
//!
//! ## Architecture
//!
//! ```text
//! handl-vm-common          handl-lib-http-client            handl-lib-http-client-std
//! ─────────────            ─────────────────────         ───────────────────────────
//! Module trait             HttpClient trait              StdHttpClient (ureq)
//! ModuleHost               HttpMethod/HttpRequest/HttpResponse
//!                          HttpError
//!                          HttpClientModule adapter
//! ```
//!
//! - **This crate** — abstract trait + module adapter + test stub.
//!   No I/O, no platform dependencies.
//! - **`handl-lib-http-client-std`** — concrete implementation using `ureq`.
//!
//! ## Usage
//!
//! ```ignore
//! use handl_vm_common::ModuleHost;
//! use handl_lib_http_client::{HttpClientModule, http_client_stub::StubHttpClient};
//!
//! let host = ModuleHost::new()
//!     .with(HttpClientModule::new(StubHttpClient::default()));
//! ```
//!
//! Expressions can then call `http.get("https://...")`, `http.post(url, body)`, etc.

mod error;
mod http_client;

pub use error::HttpError;

pub use http_client::HttpClient;
pub use http_client::HttpClientModule;
pub use http_client::HttpMethod;
pub use http_client::HttpRequest;
pub use http_client::HttpResponse;

// Re-export stub for convenience.
pub use http_client::http_client_stub::StubHttpClient;
