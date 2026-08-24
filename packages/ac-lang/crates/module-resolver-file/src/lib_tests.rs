use std::io::Write;

use crate::FileResolver;
use ac_module_resolver_core::ModuleResolver;
use ac_module_resolver_memory::MemoryModuleCache;

#[test]
fn resolve_yaml_file() {
    let dir = std::env::temp_dir().join("ac-module-resolver-file-test-yaml");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("base.yml");
    std::fs::write(&file, "$:\n  name:\n    type: string\n").unwrap();

    let resolver = FileResolver::new();
    let module = resolver.resolve("base.yml", Some(&dir)).unwrap();

    assert_eq!(module.schemas.len(), 1);
    assert_eq!(module.schemas[0].0, "name");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resolve_json_file() {
    let dir = std::env::temp_dir().join("ac-module-resolver-file-test-json");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("base.json");
    std::fs::write(&file, r#"{"$": {"name": {"type": "string"}}}"#).unwrap();

    let resolver = FileResolver::new();
    let module = resolver.resolve("base.json", Some(&dir)).unwrap();

    assert_eq!(module.schemas.len(), 1);
    assert_eq!(module.schemas[0].0, "name");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resolve_nonexistent_returns_none() {
    let resolver = FileResolver::new();
    assert!(resolver.resolve("nonexistent.yml", None).is_none());
}

#[test]
fn resolve_with_cache_hits_on_second_call() {
    let dir = std::env::temp_dir().join("ac-module-resolver-file-test-cache");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("base.yml");
    std::fs::write(&file, "$:\n  name:\n    type: string\n").unwrap();

    let resolver = FileResolver::with_cache(MemoryModuleCache::new());

    let first = resolver.resolve("base.yml", Some(&dir)).unwrap();
    let second = resolver.resolve("base.yml", Some(&dir)).unwrap();

    assert_eq!(first.schemas.len(), second.schemas.len());
    assert_eq!(first.schemas.len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resolve_sets_base_path_to_parent() {
    let dir = std::env::temp_dir().join("ac-module-resolver-file-test-basepath");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("base.yml");
    std::fs::write(&file, "$:\n  name:\n    type: string\n").unwrap();

    let resolver = FileResolver::new();
    let module = resolver.resolve("base.yml", Some(&dir)).unwrap();

    assert!(module.base_path.is_some());
    assert_eq!(module.base_path.unwrap(), dir);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resolve_no_extension_defaults_to_yaml() {
    let dir = std::env::temp_dir().join("ac-module-resolver-file-test-noext");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("base");
    let mut f = std::fs::File::create(&file).unwrap();
    f.write_all(b"$:\n  name:\n    type: string\n").unwrap();

    let resolver = FileResolver::new();
    let module = resolver.resolve("base", Some(&dir));
    assert!(module.is_some());

    std::fs::remove_dir_all(&dir).ok();
}