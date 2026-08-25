use crate::GitResolver;
use handl_module_resolver_core::ModuleResolver;

#[test]
fn resolve_non_git_path_without_base_returns_none() {
    let resolver = GitResolver::temp();
    assert!(resolver.resolve("base.yml", None).is_none());
}

#[test]
fn resolve_non_git_path_with_non_git_base_returns_none() {
    let resolver = GitResolver::temp();
    assert!(
        resolver
            .resolve("base.yml", Some(std::path::Path::new("/schemas")))
            .is_none()
    );
}
