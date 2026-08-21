//! Canned-response [`HttpClient`] stub for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::PlatformError;
use crate::http_client::{HttpClient, HttpRequest, HttpResponse};

/// An HTTP client that returns canned responses by URL.
#[derive(Default)]
pub struct StubHttpClient {
    responses: Mutex<HashMap<String, HttpResponse>>,
}

impl StubHttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(url: impl Into<String>, response: HttpResponse) -> Self {
        let mut responses = HashMap::new();
        responses.insert(url.into(), response);
        StubHttpClient {
            responses: Mutex::new(responses),
        }
    }
}

impl HttpClient for StubHttpClient {
    fn request(&self, req: &HttpRequest) -> Result<HttpResponse, PlatformError> {
        self.responses
            .lock()
            .unwrap()
            .get(&req.url)
            .cloned()
            .ok_or(PlatformError::Network {
                message: format!("no canned response for {}", req.url),
            })
    }
}
