use std::collections::HashMap;

use handl_lib_http_client::{HttpClient, HttpError, HttpMethod, HttpRequest, HttpResponse};

/// HTTP client implementation backed by [`ureq`].
#[derive(Default)]
pub struct StdHttpClient;

fn extract_headers(resp: &ureq::Response) -> HashMap<String, String> {
    resp.headers_names()
        .into_iter()
        .filter_map(|name| resp.header(&name).map(|value| (name, value.to_string())))
        .collect()
}

fn into_http_response(resp: ureq::Response) -> Result<HttpResponse, HttpError> {
    let status = resp.status();
    let status_text = resp.status_text().to_string();
    let url = resp.get_url().to_string();
    let headers = extract_headers(&resp);
    let body = resp.into_string().map_err(|e| HttpError::Network {
        message: e.to_string(),
    })?;
    Ok(HttpResponse {
        status,
        status_text,
        body,
        headers,
        url,
    })
}

fn build_request(req: &HttpRequest) -> ureq::Request {
    let mut builder = match req.method {
        HttpMethod::Get => ureq::get(&req.url),
        HttpMethod::Post => ureq::post(&req.url),
        HttpMethod::Put => ureq::put(&req.url),
        HttpMethod::Patch => ureq::request("PATCH", &req.url),
        HttpMethod::Delete => ureq::delete(&req.url),
        HttpMethod::Head => ureq::head(&req.url),
        HttpMethod::Options => ureq::request("OPTIONS", &req.url),
    };

    for (name, value) in &req.headers {
        builder = builder.set(name, value);
    }

    if let Some(ms) = req.timeout_ms {
        builder = builder.timeout(std::time::Duration::from_millis(ms));
    }

    builder
}

impl HttpClient for StdHttpClient {
    fn request(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
        let builder = build_request(req);

        let response = match &req.body {
            Some(body) => builder.send_string(body),
            None => builder.call(),
        };

        response
            .map_err(|e| HttpError::Network {
                message: e.to_string(),
            })
            .and_then(into_http_response)
    }
}
