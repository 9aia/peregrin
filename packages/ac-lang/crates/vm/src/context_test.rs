use super::*;
use ac_runtime::RuntimeValue;

#[test]
fn context_define_and_get() {
    let mut ctx = Context::new();
    ctx.define("x", RuntimeValue::new(42));
    let val = ctx.get("x").unwrap();
    assert_eq!(val.as_number(), Some(42.0));
}

#[test]
fn context_unknown_variable() {
    let ctx = Context::new();
    assert!(ctx.get("nonexistent").is_err());
}

#[test]
fn context_scope_push_pop() {
    let mut ctx = Context::new();
    ctx.define("x", RuntimeValue::new(1));
    ctx.push_scope();
    ctx.define("x", RuntimeValue::new(2));
    assert_eq!(ctx.get("x").unwrap().as_number(), Some(2.0));
    ctx.pop_scope();
    assert_eq!(ctx.get("x").unwrap().as_number(), Some(1.0));
}

#[test]
fn context_cannot_pop_global() {
    let mut ctx = Context::new();
    assert!(ctx.pop_scope().is_none());
}

#[test]
fn context_depth() {
    let mut ctx = Context::new();
    assert_eq!(ctx.depth(), 1);
    ctx.push_scope();
    assert_eq!(ctx.depth(), 2);
    ctx.pop_scope();
    assert_eq!(ctx.depth(), 1);
}

#[test]
fn context_default() {
    let ctx = Context::default();
    assert_eq!(ctx.depth(), 1);
}
