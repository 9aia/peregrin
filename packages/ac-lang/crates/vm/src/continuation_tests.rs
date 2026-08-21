use crate::context::Context;
use crate::continuation::{Continuation, EvalResult, Frame};
use crate::error::Error;
use crate::host::{Function, Host};
use ac_ast::Expr;
use ac_runtime::RuntimeValue;

// ── test hosts ───────────────────────────────────────────────────

#[derive(Clone)]
struct Add;
impl Function for Add {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(RuntimeValue::new(sum))
    }
}

#[derive(Clone)]
struct Mul;
impl Function for Mul {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let product: f64 = args.iter().filter_map(|v| v.as_number()).product();
        Ok(RuntimeValue::new(product))
    }
}

/// A host function that always yields on the first call.
#[derive(Clone)]
struct YieldingFetch;
impl Function for YieldingFetch {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let input = args
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Err(Error::Yield(RuntimeValue::new(format!("yield:{input}"))))
    }
}

/// A host function that returns cached results for specific arguments,
/// and yields for uncached arguments.
#[derive(Clone)]
struct CachingFetch {
    cache: std::collections::HashMap<String, RuntimeValue>,
}
impl Function for CachingFetch {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let key = args
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if let Some(cached) = self.cache.get(key) {
            Ok(cached.clone())
        } else {
            Err(Error::Yield(RuntimeValue::new(format!("yield:{key}"))))
        }
    }
}

// ── hosts ────────────────────────────────────────────────────────

#[derive(Clone)]
struct ServerHost {
    add: Add,
    mul: Mul,
    fetch: YieldingFetch,
}

impl Host for ServerHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            "fetch" => Some(&self.fetch),
            _ => None,
        }
    }
}

impl Default for ServerHost {
    fn default() -> Self {
        ServerHost {
            add: Add,
            mul: Mul,
            fetch: YieldingFetch,
        }
    }
}

#[derive(Clone)]
struct ClientHost {
    add: Add,
    mul: Mul,
    fetch: CachingFetch,
}

impl Host for ClientHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            "fetch" => Some(&self.fetch),
            _ => None,
        }
    }
}

impl ClientHost {
    fn new(cache: Vec<(&str, RuntimeValue)>) -> Self {
        let cache = cache
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        ClientHost {
            add: Add,
            mul: Mul,
            fetch: CachingFetch { cache },
        }
    }
}

// ── tests: basic resumable eval ──────────────────────────────────

#[test]
fn resumable_eval_value() {
    let mut vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let expr = Expr::value(RuntimeValue::new(42));
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    assert!(matches!(result, EvalResult::Done(ref v) if v.as_number() == Some(42.0)));
}

#[test]
fn resumable_eval_simple_call() {
    let mut vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(RuntimeValue::new(1)),
            Expr::value(RuntimeValue::new(2)),
        ],
    );
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    assert!(matches!(result, EvalResult::Done(ref v) if v.as_number() == Some(3.0)));
}

#[test]
fn resumable_eval_nested_calls() {
    let mut vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call(
        "mul",
        vec![
            Expr::call(
                "add",
                vec![
                    Expr::value(RuntimeValue::new(1)),
                    Expr::value(RuntimeValue::new(2)),
                ],
            ),
            Expr::value(RuntimeValue::new(3)),
        ],
    );
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    // (1 + 2) * 3 = 9
    assert!(matches!(result, EvalResult::Done(ref v) if v.as_number() == Some(9.0)));
}

// ── tests: yield + resume ────────────────────────────────────────

#[test]
fn resumable_eval_yields_on_fetch() {
    let mut vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("fetch", vec![Expr::value(RuntimeValue::new("hello"))]);
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    match result {
        EvalResult::Yielded { value, continuation } => {
            assert_eq!(value.as_str(), Some("yield:hello"));
            assert!(!continuation.is_empty());
        }
        other => panic!("expected Yielded, got {other:?}"),
    }
}

#[test]
fn resumable_eval_resume_with_different_host() {
    let expr = Expr::call(
        "add",
        vec![
            Expr::call("fetch", vec![Expr::value(RuntimeValue::new("data"))]),
            Expr::value(RuntimeValue::new(10)),
        ],
    );

    // Server: starts evaluation, fetch yields
    let mut server_vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let result = server_vm.eval_resumable(&expr, &mut ctx, None);
    let (yielded_value, continuation) = match result {
        EvalResult::Yielded { value, continuation } => (value, continuation),
        other => panic!("expected Yielded, got {other:?}"),
    };

    assert_eq!(yielded_value.as_str(), Some("yield:data"));

    // Serialize the continuation
    let json = continuation.to_json();
    let restored = Continuation::from_json(&json).unwrap();
    assert_eq!(restored, continuation);

    // Client: resume with a different host that has the cached result
    let mut client_vm = crate::Vm::new(ClientHost::new(vec![("data", RuntimeValue::new(5.0))]));
    let result = client_vm.eval_resumable(&expr, &mut ctx, Some(restored));

    // fetch returns 5, add(5, 10) = 15
    match result {
        EvalResult::Done(value) => assert_eq!(value.as_number(), Some(15.0)),
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn resumable_eval_resume_after_nested_yield() {
    // mul(fetch("a"), add(fetch("b"), 10))
    // Server evaluates: fetch("a") yields first
    let expr = Expr::call(
        "mul",
        vec![
            Expr::call("fetch", vec![Expr::value(RuntimeValue::new("a"))]),
            Expr::call(
                "add",
                vec![
                    Expr::call("fetch", vec![Expr::value(RuntimeValue::new("b"))]),
                    Expr::value(RuntimeValue::new(10)),
                ],
            ),
        ],
    );

    // Server: first yield at fetch("a")
    let mut server_vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let result = server_vm.eval_resumable(&expr, &mut ctx, None);
    let (yielded_a, cont_a) = match result {
        EvalResult::Yielded { value, continuation } => (value, continuation),
        other => panic!("expected first Yielded, got {other:?}"),
    };
    assert_eq!(yielded_a.as_str(), Some("yield:a"));

    // Client with cache for "a"=3: resume, then fetch("b") yields
    let mut client_vm = crate::Vm::new(ClientHost::new(vec![("a", RuntimeValue::new(3.0))]));
    let result = client_vm.eval_resumable(&expr, &mut ctx, Some(cont_a));
    let (yielded_b, cont_b) = match result {
        EvalResult::Yielded { value, continuation } => (value, continuation),
        other => panic!("expected second Yielded, got {other:?}"),
    };
    assert_eq!(yielded_b.as_str(), Some("yield:b"));

    // Client2 with cache for "b"=20: resume, should complete
    // mul(3, add(20, 10)) = mul(3, 30) = 90
    let mut client2_vm = crate::Vm::new(ClientHost::new(vec![("b", RuntimeValue::new(20.0))]));
    let result = client2_vm.eval_resumable(&expr, &mut ctx, Some(cont_b));
    match result {
        EvalResult::Done(value) => assert_eq!(value.as_number(), Some(90.0)),
        other => panic!("expected Done, got {other:?}"),
    }
}

// ── tests: serialization ─────────────────────────────────────────

#[test]
fn continuation_serialization_roundtrip() {
    let cont = Continuation {
        frames: vec![Frame {
            name: "add".to_string(),
            evaluated: vec![RuntimeValue::new(1), RuntimeValue::new(2)],
            pending: vec![Expr::call("fetch", vec![Expr::value(RuntimeValue::new("x"))])],
        }],
    };

    let json = cont.to_json();
    let restored = Continuation::from_json(&json).unwrap();
    assert_eq!(restored, cont);
}

#[test]
fn continuation_empty_serialization() {
    let cont = Continuation::empty();
    let json = cont.to_json();
    let restored = Continuation::from_json(&json).unwrap();
    assert_eq!(restored, cont);
    assert!(restored.is_empty());
}

#[test]
fn continuation_from_invalid_json_errors() {
    let bad = serde_json::json!({"not": "an array"});
    assert!(Continuation::from_json(&bad).is_err());
}

// ── tests: error cases ───────────────────────────────────────────

#[test]
fn resumable_eval_unknown_function() {
    let mut vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("nonexistent", vec![]);
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    assert!(matches!(result, EvalResult::Error(Error::UnknownFunction(_))));
}

#[test]
fn resumable_eval_resume_empty_continuation_errors() {
    let mut vm = crate::Vm::new(ServerHost::default());
    let mut ctx = Context::new();
    let expr = Expr::value(RuntimeValue::new(1));
    let result = vm.eval_resumable(&expr, &mut ctx, Some(Continuation::empty()));
    assert!(matches!(result, EvalResult::Error(_)));
}
