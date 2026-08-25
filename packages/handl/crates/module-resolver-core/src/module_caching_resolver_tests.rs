use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::module_caching_resolver::CachingResolver;
use crate::module_resolver::ModuleResolver;
use crate::stubs::counting_resolver_stub::CountingResolver;
use crate::stubs::module_cache_stub::TestCache;
use crate::stubs::none_resolver_stub::NoneResolver;

#[test]
fn caching_calls_inner_once_for_same_path() {
    let count = Arc::new(AtomicU32::new(0));
    let resolver = CachingResolver::with_cache(
        CountingResolver {
            count: count.clone(),
        },
        TestCache::new(),
    );

    assert!(resolver.resolve("foo", None).is_some());
    assert!(resolver.resolve("foo", None).is_some());
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn caching_calls_inner_for_different_paths() {
    let count = Arc::new(AtomicU32::new(0));
    let resolver = CachingResolver::with_cache(
        CountingResolver {
            count: count.clone(),
        },
        TestCache::new(),
    );

    resolver.resolve("foo", None);
    resolver.resolve("bar", None);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn caching_does_not_cache_none_results() {
    let resolver = CachingResolver::with_cache(NoneResolver, TestCache::new());

    assert!(resolver.resolve("foo", None).is_none());
    assert!(resolver.resolve("foo", None).is_none());
}

#[test]
fn caching_with_explicit_cache() {
    let cache = TestCache::new();
    let count = Arc::new(AtomicU32::new(0));
    let resolver = CachingResolver::with_cache(
        CountingResolver {
            count: count.clone(),
        },
        cache,
    );

    resolver.resolve("foo", None);
    resolver.resolve("foo", None);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(resolver.cache_len(), 1);
}

#[test]
fn caching_clear_cache() {
    let count = Arc::new(AtomicU32::new(0));
    let resolver = CachingResolver::with_cache(
        CountingResolver {
            count: count.clone(),
        },
        TestCache::new(),
    );

    resolver.resolve("foo", None);
    assert_eq!(resolver.cache_len(), 1);

    resolver.clear_cache();
    assert_eq!(resolver.cache_len(), 0);

    resolver.resolve("foo", None);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn caching_key_includes_base() {
    let count = Arc::new(AtomicU32::new(0));
    let resolver = CachingResolver::with_cache(
        CountingResolver {
            count: count.clone(),
        },
        TestCache::new(),
    );

    let base = Path::new("/schemas");
    resolver.resolve("foo", Some(base));
    resolver.resolve("foo", None);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}
