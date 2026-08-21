mod http_client;

pub use http_client::{HttpClient, HttpClientModule, HttpRequest, HttpResponse, HttpMethod};

pub mod http_client_stub;

#[cfg(test)]
mod http_client_tests;
