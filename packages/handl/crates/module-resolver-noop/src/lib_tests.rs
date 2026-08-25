use handl_module_resolver_core::ModuleCache;
use handl_module_resolver_core::resolved_module_test_helpers::make_resolved_module;

#[test]
fn no_cache_get_returns_none() {
    let cache = crate::NoCache;
    assert!(cache.get("anything").is_none());
}

#[test]
fn no_cache_insert_is_noop() {
    let cache = crate::NoCache;
    cache.insert("key", make_resolved_module("a"));
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn no_cache_clear() {
    let cache = crate::NoCache;
    cache.clear();
    assert_eq!(cache.len(), 0);
}
