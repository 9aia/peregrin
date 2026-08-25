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

#[test]
fn async_function_lookup_hit() {
    let m = StubModule;
    assert!(m.async_function("async_echo").is_some());
}

#[test]
fn async_function_lookup_miss() {
    let m = StubModule;
    assert!(m.async_function("nope").is_none());
}

#[test]
fn operation_lookup_hit() {
    let m = StubModule;
    assert!(m.operation("raise").is_some());
}

#[test]
fn operation_lookup_miss() {
    let m = StubModule;
    assert!(m.operation("nope").is_none());
}
