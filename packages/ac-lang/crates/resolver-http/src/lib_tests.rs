use super::*;

#[test]
fn resolve_url_absolute() {
    let url = resolve_url("https://example.com/schemas/base.yml", None);
    assert_eq!(
        url.map(|u| u.to_string()),
        Some("https://example.com/schemas/base.yml".to_string())
    );
}

#[test]
fn resolve_url_relative() {
    let base = Path::new("https://example.com/schemas/parent.yml");
    let url = resolve_url("./base.yml", Some(base));
    assert_eq!(
        url.map(|u| u.to_string()),
        Some("https://example.com/schemas/base.yml".to_string())
    );
}

#[test]
fn resolve_url_non_http_returns_none() {
    assert!(resolve_url("./base.yml", None).is_none());
    assert!(resolve_url("/etc/passwd", None).is_none());
}

#[test]
fn resolve_url_relative_without_base_returns_none() {
    assert!(resolve_url("./base.yml", None).is_none());
}
