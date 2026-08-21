use super::*;
use crate::context::Context;
use crate::error::Error;
use crate::host::{AsyncFunction, Function, Host};
use ac_ast::Expr;
use ac_runtime::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};

fn block_on<F: Future>(future: F) -> F::Output {
    fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable =
        RawWakerVTable::new(noop_clone, noop, noop, noop);

    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = TaskContext::from_waker(&waker);

    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(val) = future.as_mut().poll(&mut cx) {
            return val;
        }
        std::thread::yield_now();
    }
}

#[derive(Clone)]
struct Add;
impl Function for Add {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(RuntimeValue::new(sum))
    }
}

#[derive(Clone)]
struct DefaultHost {
    add: Add,
}

impl Host for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }
}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost { add: Add }
    }
}

#[test]
fn vm_eval_value() {
    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();
    let expr = Expr::value(RuntimeValue::new(42));
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn vm_eval_call() {
    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("add", vec![Expr::value(RuntimeValue::new(1)), Expr::value(RuntimeValue::new(2))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn vm_eval_unknown_function() {
    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("nonexistent", vec![]);
    let result = vm.eval(&expr, &mut ctx);
    assert!(result.is_err());
}

#[test]
fn vm_host_accessor() {
    let vm = Vm::new(DefaultHost::default());
    assert!(vm.host().function("add").is_some());
    assert!(vm.host().function("nope").is_none());
}

// ── async tests ──────────────────────────────────────────────────

#[derive(Clone)]
struct AsyncFetch;
impl AsyncFunction for AsyncFetch {
    fn call<'a>(
        &'a self,
        args: &'a [RuntimeValue],
        _ctx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = Result<RuntimeValue, Error>> + 'a>> {
        Box::pin(async {
            let input = args.first().and_then(|v| v.as_str()).unwrap_or("");
            Ok(RuntimeValue::new(format!("fetched:{input}")))
        })
    }
}

#[derive(Clone)]
struct AsyncAdd;
impl AsyncFunction for AsyncAdd {
    fn call<'a>(
        &'a self,
        args: &'a [RuntimeValue],
        _ctx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = Result<RuntimeValue, Error>> + 'a>> {
        Box::pin(async {
            let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
            Ok(RuntimeValue::new(sum))
        })
    }
}

#[derive(Clone)]
struct YieldOnce;
impl AsyncFunction for YieldOnce {
    fn call<'a>(
        &'a self,
        args: &'a [RuntimeValue],
        _ctx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = Result<RuntimeValue, Error>> + 'a>> {
        Box::pin(async {
            let val = args.first().cloned().unwrap_or(RuntimeValue::new(0));
            Err(Error::Yield(val))
        })
    }
}

#[derive(Clone)]
struct MixedHost {
    add: Add,
    async_add: AsyncAdd,
    fetch: AsyncFetch,
    yield_once: YieldOnce,
}

impl Host for MixedHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        match name {
            "async_add" => Some(&self.async_add),
            "fetch" => Some(&self.fetch),
            "yield_once" => Some(&self.yield_once),
            _ => None,
        }
    }
}

impl Default for MixedHost {
    fn default() -> Self {
        MixedHost {
            add: Add,
            async_add: AsyncAdd,
            fetch: AsyncFetch,
            yield_once: YieldOnce,
        }
    }
}

#[test]
fn vm_eval_async_value() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::value(RuntimeValue::new(42));
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn vm_eval_async_call_sync_function() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(RuntimeValue::new(1)),
            Expr::value(RuntimeValue::new(2)),
        ],
    );
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn vm_eval_async_call_async_function() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call(
        "async_add",
        vec![
            Expr::value(RuntimeValue::new(10)),
            Expr::value(RuntimeValue::new(20)),
        ],
    );
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(30.0));
}

#[test]
fn vm_eval_async_nested_sync_in_async() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call(
        "async_add",
        vec![
            Expr::call(
                "add",
                vec![
                    Expr::value(RuntimeValue::new(1)),
                    Expr::value(RuntimeValue::new(2)),
                ],
            ),
            Expr::value(RuntimeValue::new(20)),
        ],
    );
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(23.0));
}

#[test]
fn vm_eval_async_fetch() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("fetch", vec![Expr::value(RuntimeValue::new("hello"))]);
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_str(), Some("fetched:hello"));
}

#[test]
fn vm_eval_async_yield() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("yield_once", vec![Expr::value(RuntimeValue::new(99))]);
    let result = block_on(vm.eval_async(&expr, &mut ctx));
    match result {
        Err(Error::Yield(val)) => assert_eq!(val.as_number(), Some(99.0)),
        other => panic!("expected Yield, got {other:?}"),
    }
}

#[test]
fn vm_eval_async_unknown_function() {
    let mut vm = Vm::new(MixedHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("nonexistent", vec![]);
    let result = block_on(vm.eval_async(&expr, &mut ctx));
    assert!(matches!(result, Err(Error::UnknownFunction(_))));
}
