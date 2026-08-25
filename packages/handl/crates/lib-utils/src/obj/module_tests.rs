use super::test_helpers::*;

// ── module ─────────────────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(ObjModule.name(), "obj");
}

#[test]
fn unknown_function_returns_none() {
    assert!(ObjModule.function("nope").is_none());
}
