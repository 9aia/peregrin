use super::{CacheHost, CallCache};
use handl_ast_canonical::Expr;
use handl_vm::NonFunctionValue;
use handl_vm_backend_rust_interpreter::{
    CanonicalContext, Context, Error, EvalResult, Function, Host, Vm,
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct EmptyHost;
impl Host<NonFunctionValue> for EmptyHost {
    fn function(&self, _name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        None
    }
}

// ── test functions ───────────────────────────────────────────────

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

struct CountingFetch {
    calls: AtomicUsize,
}
impl Function<NonFunctionValue> for CountingFetch {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let input = args.first().and_then(|v| v.as_str()).unwrap_or("?");
        Ok(NonFunctionValue::new(format!("result_{input}_{n}")))
    }
}

struct YieldingFetch;
impl Function<NonFunctionValue> for YieldingFetch {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let input = args.first().and_then(|v| v.as_str()).unwrap_or("?");
        Err(Error::Yield(NonFunctionValue::new(format!(
            "yield:{input}"
        ))))
    }
}

// ── cache tests ──────────────────────────────────────────────────

#[test]
fn cache_insert_and_get() {
    let mut cache = CallCache::new();
    cache.insert(
        "add",
        &[NonFunctionValue::new(1), NonFunctionValue::new(2)],
        NonFunctionValue::new(3),
    );
    assert_eq!(cache.len(), 1);
    let result = cache.get("add", &[NonFunctionValue::new(1), NonFunctionValue::new(2)]);
    assert_eq!(result.unwrap().as_number(), Some(3.0));
}

#[test]
fn cache_miss_returns_none() {
    let cache = CallCache::new();
    assert!(cache.get("add", &[NonFunctionValue::new(1)]).is_none());
}

#[test]
fn cache_serialization_roundtrip() {
    let mut cache = CallCache::new();
    cache.insert(
        "add",
        &[NonFunctionValue::new(1), NonFunctionValue::new(2)],
        NonFunctionValue::new(3),
    );
    cache.insert(
        "fetch",
        &[NonFunctionValue::new("hello")],
        NonFunctionValue::new("result_hello_0"),
    );

    let json = cache.to_json();
    let restored = CallCache::from_json(&json);

    assert_eq!(restored.len(), 2);
    assert_eq!(
        restored
            .get("add", &[NonFunctionValue::new(1), NonFunctionValue::new(2)])
            .unwrap()
            .as_number(),
        Some(3.0),
    );
    assert_eq!(
        restored
            .get("fetch", &[NonFunctionValue::new("hello")])
            .unwrap()
            .as_str(),
        Some("result_hello_0"),
    );
}

#[test]
fn cache_empty_serialization() {
    let cache = CallCache::new();
    let json = cache.to_json();
    let restored = CallCache::from_json(&json);
    assert!(restored.is_empty());
}

// ── CacheHost tests ──────────────────────────────────────────────

#[test]
fn cache_host_caches_successful_calls() {
    let mut host = CacheHost::new(EmptyHost);
    let fetch = CountingFetch {
        calls: AtomicUsize::new(0),
    };
    host.register("fetch", Box::new(fetch));
    host.register("add", Box::new(Add));

    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();

    // First call: cache miss, dispatches to function
    let expr = Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("a"))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("result_a_0"));

    // Second call: cache hit, returns cached result
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("result_a_0"));

    // Verify the function was only called once
    let cache = vm.host().cache();
    assert_eq!(cache.len(), 1);
}

#[test]
fn cache_host_does_not_cache_yields() {
    let mut host = CacheHost::new(EmptyHost);
    host.register("fetch", Box::new(YieldingFetch));
    host.register("add", Box::new(Add));

    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();

    let expr = Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("a"))]);
    let result = vm.eval(&expr, &mut ctx);
    assert!(matches!(result, Err(Error::Yield(_))));

    // Yield should not be cached
    let cache = vm.host().cache();
    assert!(cache.is_empty());
}

// ── end-to-end: server → client migration ────────────────────────

#[test]
fn end_to_end_server_to_client_migration() {
    // Expression: add(fetch("data"), 10)
    let expr = Expr::call(
        "add",
        vec![
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
            Expr::value(NonFunctionValue::new(10)),
        ],
    );

    // ── Server ───────────────────────────────────────────────────
    let mut server_host = CacheHost::new(EmptyHost);
    server_host.register("add", Box::new(Add));
    server_host.register("fetch", Box::new(YieldingFetch));

    let mut server_vm = Vm::new(server_host);
    let mut ctx = CanonicalContext::new();

    let result = server_vm.eval_resumable(&expr, &mut ctx, None);
    let (yielded_value, continuation) = match result {
        EvalResult::Yielded {
            value,
            continuation,
        } => (value, continuation),
        other => panic!("expected Yielded, got {other:?}"),
    };
    assert_eq!(yielded_value.as_str(), Some("yield:data"));

    // Serialize continuation + cache
    let cont_json = continuation.to_json();
    let cache_json = server_vm.host().cache().to_json();

    // ── Client ───────────────────────────────────────────────────
    let cache = CallCache::from_json(&cache_json);
    let mut client_host = CacheHost::with_cache(EmptyHost, cache);

    // Pre-populate the cache with the fetch result (simulating
    // the client knowing the result of the yielded call)
    client_host.cache_mut().insert(
        "fetch",
        &[NonFunctionValue::new("data")],
        NonFunctionValue::new(42.0),
    );

    client_host.register("add", Box::new(Add));
    client_host.register("fetch", Box::new(YieldingFetch));

    let mut client_vm = Vm::new(client_host);
    let restored_cont =
        handl_vm_backend_rust_interpreter::Continuation::from_json(&cont_json).unwrap();

    let result = client_vm.eval_resumable(&expr, &mut ctx, Some(restored_cont));
    match result {
        EvalResult::Done(value) => assert_eq!(value.as_number(), Some(52.0)),
        other => panic!("expected Done, got {other:?}"),
    }
}
