use crate::HttpResolver;
use ac_module_resolver_core::ModuleResolver;

#[test]
fn resolve_non_url_path_without_base_returns_none() {
    let resolver = HttpResolver::new();
    assert!(resolver.resolve("base.yml", None).is_none());
}

#[test]
fn resolve_absolute_non_url_returns_none() {
    let resolver = HttpResolver::new();
    assert!(resolver.resolve("/etc/passwd", None).is_none());
}