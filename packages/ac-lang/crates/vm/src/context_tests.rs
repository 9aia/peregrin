use super::*;
use ac_runtime::NonFunctionValue;
use std::future::Future;
use std::pin::Pin;

#[test]
fn context_define_and_get() {
    let mut ctx = Context::new();
    ctx.define("x", NonFunctionValue::new(42));
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
    ctx.define("x", NonFunctionValue::new(1));
    ctx.push_scope();
    ctx.define("x", NonFunctionValue::new(2));
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

#[test]
fn context_has_eval_false_by_default() {
    let ctx = Context::new();
    assert!(!ctx.has_eval());
}

#[test]
fn context_eval_expr_errors_without_callback() {
    let ctx = Context::new();
    let result = ctx.eval_expr("x", &[]);
    assert!(result.is_err());
}

#[test]
fn context_eval_expr_with_callback() {
    struct DoubleEval;
    impl crate::Eval for DoubleEval {
        fn eval_expr(
            &self,
            _expr: &str,
            vars: &[(String, NonFunctionValue)],
        ) -> Result<NonFunctionValue, crate::Error> {
            let val = vars
                .first()
                .map(|(_, v)| v.as_number().unwrap_or(0.0))
                .unwrap_or(0.0);
            Ok(NonFunctionValue::new(val * 2.0))
        }
    }

    let mut ctx = Context::new();
    ctx.set_eval(Box::new(DoubleEval));
    assert!(ctx.has_eval());
    let result = ctx.eval_expr("x", &[(String::from("x"), NonFunctionValue::new(21))]);
    assert_eq!(result.unwrap().as_number(), Some(42.0));
}

#[test]
fn context_has_async_eval_false_by_default() {
    let ctx = Context::new();
    assert!(!ctx.has_async_eval());
}

#[test]
fn context_eval_expr_async_errors_without_callback() {
    let ctx = Context::new();
    let fut = ctx.eval_expr_async("x", &[]);
    let waker = noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    let mut pinned = std::pin::pin!(fut);
    let poll = pinned.as_mut().poll(&mut cx);
    assert!(poll.is_ready());
    if let std::task::Poll::Ready(result) = poll {
        assert!(result.is_err());
    }
}

#[test]
fn context_eval_expr_async_with_callback() {
    struct DoubleAsyncEval;
    impl crate::AsyncEval for DoubleAsyncEval {
        fn eval_expr_async<'a>(
            &'a self,
            _expr: &'a str,
            vars: &'a [(String, NonFunctionValue)],
        ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, crate::Error>> + 'a>> {
            Box::pin(async {
                let val = vars
                    .first()
                    .map(|(_, v)| v.as_number().unwrap_or(0.0))
                    .unwrap_or(0.0);
                Ok(NonFunctionValue::new(val * 2.0))
            })
        }
    }

    let mut ctx = Context::new();
    ctx.set_async_eval(Box::new(DoubleAsyncEval));
    assert!(ctx.has_async_eval());

    let vars = [(String::from("x"), NonFunctionValue::new(21))];
    let fut = ctx.eval_expr_async("x", &vars);
    let mut pinned = std::pin::pin!(fut);
    let waker = noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        if let std::task::Poll::Ready(result) = pinned.as_mut().poll(&mut cx) {
            assert_eq!(result.unwrap().as_number(), Some(42.0));
            return;
        }
        std::thread::yield_now();
    }
}

#[test]
fn context_collect_vars() {
    let mut ctx = Context::new();
    ctx.define("x", NonFunctionValue::new(1));
    ctx.define("y", NonFunctionValue::new(2));
    ctx.push_scope();
    ctx.define("x", NonFunctionValue::new(10));
    ctx.define("z", NonFunctionValue::new(3));

    let vars = ctx.collect_vars();
    let map: std::collections::HashMap<String, NonFunctionValue> = vars.into_iter().collect();
    assert_eq!(map.get("x").unwrap().as_number(), Some(10.0));
    assert_eq!(map.get("y").unwrap().as_number(), Some(2.0));
    assert_eq!(map.get("z").unwrap().as_number(), Some(3.0));
}

fn noop_waker() -> std::task::Waker {
    use std::task::{RawWaker, RawWakerVTable};
    fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);
    unsafe { std::task::Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}
