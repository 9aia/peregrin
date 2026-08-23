use crate::module_resolver::ModuleResolver;
use crate::resolved_module::ResolvedModule;

use std::path::PathBuf;

use serde_json::json;

#[test]
fn resolved_module_construction() {
    let module = ResolvedModule {
        schemas: vec![("base".to_string(), json!({"type": "object"}))],
        merge: vec!["base".to_string()],
        data: json!({"version": 1}),
        base_path: Some(PathBuf::from("/schemas"))};

    assert_eq!(module.schemas.len(), 1);
    assert_eq!(module.merge, vec!["base".to_string()]);
    assert_eq!(module.data, json!({"version": 1}));
    assert_eq!(module.base_path, Some(PathBuf::from("/schemas")));
}

#[test]
fn resolved_module_clone() {
    let module = ResolvedModule {
        schemas: vec![("base".to_string(), json!({"type": "object"}))],
        merge: vec![],
        data: json!(null),
        base_path: None};
    let cloned = module.clone();
    assert_eq!(module.schemas.len(), cloned.schemas.len());
}

#[test]
fn module_resolver_trait_is_object_safe() {
    // Verify ModuleResolver can be used as a trait object.
    struct DummyResolver;
    impl ModuleResolver for DummyResolver {
        fn resolve(&self, _path: &str, _base: Option<&std::path::Path>) -> Option<ResolvedModule> {
            None
        }
    }
    let _boxed: Box<dyn ModuleResolver> = Box::new(DummyResolver);
}
