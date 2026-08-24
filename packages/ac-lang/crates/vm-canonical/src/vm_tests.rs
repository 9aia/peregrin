use super::*;
use crate::CanonicalContext;
use crate::Context;
use crate::Error;
use crate::Expr;
use crate::NonFunctionValue;
use crate::{AsyncFunction, Function, Host};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};

fn block_on<F: Future>(future: F) -> F::Output {
    fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

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
impl Function<NonFunctionValue> for Add {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(sum))
    }
}

#[derive(Clone)]
struct DefaultHost {
    add: Add,
}

impl Host<NonFunctionValue> for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
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
    let vm = Vm::new(DefaultHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::value(NonFunctionValue::new(42));
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn vm_eval_call() {
    let vm = Vm::new(DefaultHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(NonFunctionValue::new(1)),
            Expr::value(NonFunctionValue::new(2)),
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn vm_eval_unknown_function() {
    let vm = Vm::new(DefaultHost::default());
    let mut ctx = CanonicalContext::new();
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
impl AsyncFunction<NonFunctionValue> for AsyncFetch {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>> {
        Box::pin(async {
            let input = args.first().and_then(|v| v.as_str()).unwrap_or("");
            Ok(NonFunctionValue::new(format!("fetched:{input}")))
        })
    }
}

#[derive(Clone)]
struct AsyncAdd;
impl AsyncFunction<NonFunctionValue> for AsyncAdd {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>> {
        Box::pin(async {
            let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
            Ok(NonFunctionValue::new(sum))
        })
    }
}

#[derive(Clone)]
struct YieldOnce;
impl AsyncFunction<NonFunctionValue> for YieldOnce {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> + 'a>> {
        Box::pin(async {
            let val = args.first().cloned().unwrap_or(NonFunctionValue::new(0));
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

impl Host<NonFunctionValue> for MixedHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
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
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::value(NonFunctionValue::new(42));
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn vm_eval_async_call_sync_function() {
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(NonFunctionValue::new(1)),
            Expr::value(NonFunctionValue::new(2)),
        ],
    );
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn vm_eval_async_call_async_function() {
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call(
        "async_add",
        vec![
            Expr::value(NonFunctionValue::new(10)),
            Expr::value(NonFunctionValue::new(20)),
        ],
    );
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(30.0));
}

#[test]
fn vm_eval_async_nested_sync_in_async() {
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call(
        "async_add",
        vec![
            Expr::call(
                "add",
                vec![
                    Expr::value(NonFunctionValue::new(1)),
                    Expr::value(NonFunctionValue::new(2)),
                ],
            ),
            Expr::value(NonFunctionValue::new(20)),
        ],
    );
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_number(), Some(23.0));
}

#[test]
fn vm_eval_async_fetch() {
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("hello"))]);
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_str(), Some("fetched:hello"));
}

#[test]
fn vm_eval_async_yield() {
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call("yield_once", vec![Expr::value(NonFunctionValue::new(99))]);
    let result = block_on(vm.eval_async(&expr, &mut ctx));
    match result {
        Err(Error::Yield(val)) => assert_eq!(val.as_number(), Some(99.0)),
        other => panic!("expected Yield, got {other:?}"),
    }
}

#[test]
fn vm_eval_async_unknown_function() {
    let vm = Vm::new(MixedHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call("nonexistent", vec![]);
    let result = block_on(vm.eval_async(&expr, &mut ctx));
    assert!(matches!(result, Err(Error::UnknownFunction(_))));
}

// ── deeply nested sync eval ───────────────────────────────────────

#[test]
fn vm_eval_deeply_nested_calls() {
    // add(add(add(1, 2), add(3, 4)), add(5, 6)) = 21
    let expr = Expr::call(
        "add",
        vec![
            Expr::call(
                "add",
                vec![
                    Expr::call(
                        "add",
                        vec![
                            Expr::value(NonFunctionValue::new(1)),
                            Expr::value(NonFunctionValue::new(2)),
                        ],
                    ),
                    Expr::call(
                        "add",
                        vec![
                            Expr::value(NonFunctionValue::new(3)),
                            Expr::value(NonFunctionValue::new(4)),
                        ],
                    ),
                ],
            ),
            Expr::call(
                "add",
                vec![
                    Expr::value(NonFunctionValue::new(5)),
                    Expr::value(NonFunctionValue::new(6)),
                ],
            ),
        ],
    );
    let vm = Vm::new(DefaultHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(21.0));
}

#[test]
fn vm_eval_call_with_no_args() {
    struct GetAnswer;
    impl Function<NonFunctionValue> for GetAnswer {
        fn call(
            &self,
            _args: &[NonFunctionValue],
            _ctx: &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
            Ok(NonFunctionValue::new(42))
        }
    }
    struct AnswerHost {
        answer: GetAnswer,
    }
    impl Host<NonFunctionValue> for AnswerHost {
        fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
            match name {
                "answer" => Some(&self.answer),
                _ => None,
            }
        }
    }
    impl Default for AnswerHost {
        fn default() -> Self {
            AnswerHost { answer: GetAnswer }
        }
    }

    let vm = Vm::new(AnswerHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call("answer", vec![]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn vm_eval_value_types() {
    let vm = Vm::new(DefaultHost::default());
    let mut ctx = CanonicalContext::new();

    // String
    let result = vm
        .eval(&Expr::value(NonFunctionValue::new("hello")), &mut ctx)
        .unwrap();
    assert_eq!(result.as_str(), Some("hello"));

    // Boolean
    let result = vm
        .eval(&Expr::value(NonFunctionValue::new(true)), &mut ctx)
        .unwrap();
    assert_eq!(result.as_bool(), Some(true));

    // Null
    let result = vm
        .eval(
            &Expr::value(NonFunctionValue::new(serde_json::Value::Null)),
            &mut ctx,
        )
        .unwrap();
    assert!(result.is_null());

    // Array
    let result = vm
        .eval(&Expr::value(NonFunctionValue::new(vec![1, 2, 3])), &mut ctx)
        .unwrap();
    assert_eq!(result.as_array().unwrap().len(), 3);

    // Object
    let result = vm
        .eval(
            &Expr::value(NonFunctionValue::new(serde_json::json!({"key": "val"}))),
            &mut ctx,
        )
        .unwrap();
    assert_eq!(result.get("key").unwrap().as_str(), Some("val"));
}

// ── handle via eval (not just resumable) ──────────────────────────

#[derive(Clone)]
struct PingOp;
impl crate::Operation<NonFunctionValue> for PingOp {
    fn signature(&self) -> crate::OperationSig<NonFunctionValue> {
        crate::OperationSig::<NonFunctionValue> {
            name: "ping".to_string(),
            params: vec![],
            return_schema: None,
        }
    }
    fn perform(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        Err(Error::Operation {
            name: "ping".to_string(),
            args: vec![],
        })
    }
}

#[derive(Clone)]
struct PingHost {
    add: Add,
    ping: PingOp,
}
impl Host<NonFunctionValue> for PingHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }
    fn operation(&self, name: &str) -> Option<&dyn crate::Operation<NonFunctionValue>> {
        match name {
            "ping" => Some(&self.ping),
            _ => None,
        }
    }
}
impl Default for PingHost {
    fn default() -> Self {
        PingHost {
            add: Add,
            ping: PingOp,
        }
    }
}

#[test]
fn vm_eval_handle_intercepts_via_eval() {
    use crate::HandlerClause;
    let expr = Expr::handle(
        Expr::call("ping", vec![]),
        vec![HandlerClause {
            op_name: "ping".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("pong")),
        }],
    );
    let vm = Vm::new(PingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("pong"));
}

#[test]
fn vm_eval_handle_resume_via_eval() {
    use crate::HandlerClause;
    // handle(add(ping(), 10)) — handler resumes k with 42 → add(42, 10) = 52
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("ping", vec![]),
                Expr::value(NonFunctionValue::new(10)),
            ],
        ),
        vec![HandlerClause {
            op_name: "ping".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
        }],
    );
    let vm = Vm::new(PingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(52.0));
}

#[test]
fn vm_eval_handle_no_match_propagates_as_operation_error() {
    use crate::HandlerClause;
    let expr = Expr::handle(
        Expr::call("ping", vec![]),
        vec![HandlerClause {
            op_name: "other".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("unused")),
        }],
    );
    let vm = Vm::new(PingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx);
    assert!(matches!(result, Err(Error::Operation { ref name, .. }) if name == "ping"));
}

#[test]
fn vm_eval_handle_pure_body_returns_body_value() {
    use crate::HandlerClause;
    let expr = Expr::handle(
        Expr::value(NonFunctionValue::new(99)),
        vec![HandlerClause {
            op_name: "ping".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("never")),
        }],
    );
    let vm = Vm::new(PingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(99.0));
}

// ── async eval with handle ────────────────────────────────────────

#[test]
fn vm_eval_async_handle_intercepts() {
    use crate::HandlerClause;
    let expr = Expr::handle(
        Expr::call("ping", vec![]),
        vec![HandlerClause {
            op_name: "ping".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("async-pong")),
        }],
    );
    let vm = Vm::new(PingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = block_on(vm.eval_async(&expr, &mut ctx)).unwrap();
    assert_eq!(result.as_str(), Some("async-pong"));
}
