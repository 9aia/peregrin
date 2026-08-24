//! # ac-lib-http-server-std
//!
//! Concrete HTTP server implementation backed by `tokio` and `hyper`.
//!
//! Provides [`StdHttpServer`] and a preconfigured
//! [`StdHttpServerModule`] ready for use with `ModuleHost`.

mod http_server;

pub use ac_lib_http_server::{
    HttpMethod, HttpServer, HttpServerError, HttpServerRequest, HttpServerResponse,
};
pub use http_server::{ServerConfig, StdHttpServer, StdHttpServerModule, std_http_server_module};
