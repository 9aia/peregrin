use std::path::Path;
use url::Url;

/// Resolve a path to a full URL.
///
/// If `path` is already a full URL, use it directly.
/// If `base` is provided and is a URL, resolve `path` relative to it.
pub fn resolve_url(path: &str, base: Option<&Path>) -> Option<Url> {
    if let Ok(url) = Url::parse(path)
        && url.has_host()
    {
        return Some(url);
    }

    if let Some(base) = base {
        let base_str = base.to_str()?;
        if let Ok(base_url) = Url::parse(base_str) {
            return base_url.join(path).ok();
        }
    }

    None
}
