use crate::VirtualResolver;
use ac_module_resolver_core::ModuleResolver;

use serde_json::json;

#[test]
fn resolve_unknown_path_returns_none() {
    let resolver = VirtualResolver::new();
    assert!(resolver.resolve("nonexistent", None).is_none());
}

#[test]
fn resolve_added_module() {
    let mut resolver = VirtualResolver::new();
    resolver.add("base", json!({"fields": {"name": {"type": "string"}}}));

    let module = resolver.resolve("base", None).unwrap();
    assert_eq!(module.schemas.len(), 1);
    assert_eq!(module.schemas[0].0, "name");
}

#[test]
fn resolve_added_raw_module() {
    let mut resolver = VirtualResolver::new();
    resolver.add_raw("base", "fields:\n  name:\n    type: string\n");

    let module = resolver.resolve("base", None).unwrap();
    assert_eq!(module.schemas.len(), 1);
    assert_eq!(module.schemas[0].0, "name");
}

#[test]
fn resolve_multiple_modules() {
    let mut resolver = VirtualResolver::new();
    resolver.add("a", json!({"fields": {"x": {"type": "string"}}}));
    resolver.add("b", json!({"fields": {"y": {"type": "number"}}}));

    let a = resolver.resolve("a", None).unwrap();
    let b = resolver.resolve("b", None).unwrap();

    assert_eq!(a.schemas[0].0, "x");
    assert_eq!(b.schemas[0].0, "y");
}

#[test]
fn resolve_base_path_is_none() {
    let mut resolver = VirtualResolver::new();
    resolver.add("base", json!({"fields": {"name": {"type": "string"}}}));

    let module = resolver.resolve("base", None).unwrap();
    assert!(module.base_path.is_none());
}

#[test]
fn default_creates_empty_resolver() {
    let resolver = VirtualResolver::default();
    assert!(resolver.resolve("anything", None).is_none());
}
