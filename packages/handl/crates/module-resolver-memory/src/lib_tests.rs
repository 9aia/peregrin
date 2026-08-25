use handl_module_resolver_core::ModuleCache;
use handl_module_resolver_core::resolved_module_test_helpers::make_resolved_module;

#[test]
fn memory_cache_insert_and_get() {
    let cache = crate::MemoryModuleCache::new();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());

    cache.insert("key1", make_resolved_module("a"));
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());

    let got = cache.get("key1");
    assert!(got.is_some());
    assert_eq!(got.unwrap().schemas.len(), 1);
}

#[test]
fn memory_cache_miss_returns_none() {
    let cache = crate::MemoryModuleCache::new();
    assert!(cache.get("nonexistent").is_none());
}

#[test]
fn memory_cache_clear() {
    let cache = crate::MemoryModuleCache::new();
    cache.insert("key1", make_resolved_module("a"));
    cache.insert("key2", make_resolved_module("b"));
    assert_eq!(cache.len(), 2);

    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
    assert!(cache.get("key1").is_none());
}

#[test]
fn memory_cache_overwrite() {
    let cache = crate::MemoryModuleCache::new();
    cache.insert("key", make_resolved_module("a"));
    cache.insert("key", make_resolved_module("b"));
    assert_eq!(cache.len(), 1);

    let got = cache.get("key").unwrap();
    assert_eq!(got.schemas[0].0, "b");
}

#[test]
fn memory_cache_default() {
    let cache = crate::MemoryModuleCache::default();
    assert!(cache.is_empty());
}
