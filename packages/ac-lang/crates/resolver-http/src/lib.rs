//! # ac-resolver-http
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

use std::path::Path;

use ac_fields::extract_fields;
use ac_frontend::frontend_for;
use ac_resolver_core::{ModuleResolver, ResolvedModule};
use url::Url;

/// HTTP-based module resolver.
///
/// Fetches documents via HTTP GET. Module paths must be valid URLs.
/// Relative paths are resolved against the base URL when provided.
pub struct HttpResolver;

impl ModuleResolver for HttpResolver {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let url = resolve_url(path, base)?;
        let content = ureq::get(url.as_str())
            .call()
            .ok()
            .and_then(|resp| resp.into_string().ok())?;

        let format = url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext))
            .unwrap_or("yaml");

        let frontend = frontend_for(format).ok()?;
        let value = frontend.parse(&content).ok()?;
        let extracted = extract_fields(value).ok()?;

        Some(ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: None,
        })
    }
}

/// Resolve a path to a full URL.
///
/// If `path` is already a full URL, use it directly.
/// If `base` is provided and is a URL, resolve `path` relative to it.
fn resolve_url(path: &str, base: Option<&Path>) -> Option<Url> {
    if let Ok(url) = Url::parse(path) {
        if url.has_host() {
            return Some(url);
        }
    }

    if let Some(base) = base {
        let base_str = base.to_str()?;
        if let Ok(base_url) = Url::parse(base_str) {
            return base_url.join(path).ok();
        }
    }

    None
}

#[cfg(test)]
mod tests;
