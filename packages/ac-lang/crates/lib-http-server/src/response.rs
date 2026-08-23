use std::collections::HashMap;

/// HTTP response.
#[derive(Debug, Clone)]
pub struct HttpServerResponse {
    pub status: u16,
    pub status_text: String,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl HttpServerResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        HttpServerResponse {
            status,
            status_text: String::new(),
            body: body.into(),
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_status_text(mut self, text: impl Into<String>) -> Self {
        self.status_text = text.into();
        self
    }

    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}
