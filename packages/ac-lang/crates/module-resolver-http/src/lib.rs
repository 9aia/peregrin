//! # ac-module-resolver-http
//!
//! HTTP-based [`ModuleResolver`] implementation.
//!
//! Fetches documents over HTTP(S) using `ureq`. Module paths are URLs:
//!
//! ```text
//! import('https://raw.githubusercontent.com/org/repo/main/schemas/base.yml')
//! ```
//!
//! Relative paths are resolved against the base URL (the URL of the
//! importing document).

mod http_resolver;
mod url;

pub use http_resolver::HttpResolver;

#[cfg(test)]
mod lib_tests;
#[cfg(test)]
mod url_tests;
