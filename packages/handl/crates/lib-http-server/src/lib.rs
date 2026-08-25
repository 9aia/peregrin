//! # handl-lib-http-server
//!
//! Abstract HTTP server service trait and module adapter for the handl runtime.
//!
//! Defines an abstract HTTP server interface and exposes `http_server.*`
//! functions to handl programs. No I/O and no platform dependencies in this
//! crate — concrete servers live in `handl-lib-http-server-std`.
//!
//! ## Architecture
//!
//! ```text
//! handl-vm-common          handl-lib-http-server            handl-lib-http-server-std
//! ─────────────            ─────────────────────         ───────────────────────────
//! Module trait             HttpServer trait              StdHttpServer
//! ModuleHost               HttpServerRequest/Response
//!                          HttpServerError
//!                          HttpServerModule adapter
//! ```
//!
//! - **This crate** — abstract trait + module adapter + test stub.
//! - **`handl-lib-http-server-std`** — concrete implementation.

mod error;
mod http_method;
mod http_server;
mod request;
mod response;

pub use error::HttpServerError;
pub use http_method::HttpMethod;
pub use http_server::{Handler, HttpServer, HttpServerModule, StubHttpServer};
pub use request::HttpServerRequest;
pub use response::HttpServerResponse;
