mod http_client;

pub use http_client::{HttpClient, HttpClientModule, HttpMethod, HttpRequest, HttpResponse};

pub mod http_client_stub;

#[cfg(test)]
mod http_client_tests;