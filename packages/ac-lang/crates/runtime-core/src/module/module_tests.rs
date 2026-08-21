use super::module_stub::StubModule;

use super::Module;

#[test]
fn module_name() {
    let m = StubModule;
    assert_eq!(m.name(), "stub");
}

#[test]
fn function_lookup_hit() {
    let m = StubModule;
    assert!(m.function("echo").is_some());
}

#[test]
fn function_lookup_miss() {
    let m = StubModule;
    assert!(m.function("nope").is_none());
}
