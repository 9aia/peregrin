use std::collections::HashMap;

use crate::http_method::HttpMethod;

/// HTTP request descriptor passed to route handlers.
#[derive(Debug, Clone)]
pub struct HttpServerRequest {
    pub method: HttpMethod,
    pub path: String,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpServerRequest {
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        HttpServerRequest {
            method,
            path: path.into(),
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }
}
