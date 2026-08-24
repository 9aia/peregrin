use crate::module_chain_resolver::ChainResolver;
use crate::module_resolver::ModuleResolver;
use crate::resolved_module_test_helpers::make_resolved_module;
use crate::stubs::static_resolver_stub::StaticResolver;

#[test]
fn chain_returns_first_non_none() {
    let chain = ChainResolver::new(vec![
        Box::new(StaticResolver(None)),
        Box::new(StaticResolver(Some(make_resolved_module("a")))),
        Box::new(StaticResolver(Some(make_resolved_module("b")))),
    ]);

    let result = chain.resolve("foo", None);
    assert!(result.is_some());
    assert_eq!(result.unwrap().schemas[0].0, "a");
}

#[test]
fn chain_returns_none_when_all_none() {
    let chain = ChainResolver::new(vec![
        Box::new(StaticResolver(None)),
        Box::new(StaticResolver(None)),
    ]);

    assert!(chain.resolve("foo", None).is_none());
}

#[test]
fn chain_first_resolver_wins() {
    let chain = ChainResolver::new(vec![
        Box::new(StaticResolver(Some(make_resolved_module("first")))),
        Box::new(StaticResolver(Some(make_resolved_module("second")))),
    ]);

    let result = chain.resolve("foo", None).unwrap();
    assert_eq!(result.schemas[0].0, "first");
}

#[test]
fn chain_empty_returns_none() {
    let chain = ChainResolver::new(vec![]);
    assert!(chain.resolve("foo", None).is_none());
}