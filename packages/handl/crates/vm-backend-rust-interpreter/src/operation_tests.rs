use crate::CanonicalContext;
use crate::Context;
use crate::Error;
use crate::Expr;
use crate::NonFunctionValue;
use crate::{Continuation, EvalResult};
use crate::{Function, Host};
use crate::{Operation, OperationSig};

// ── test operations ──────────────────────────────────────────────

#[derive(Clone)]
struct FetchOp;

impl Operation<NonFunctionValue> for FetchOp {
    fn signature(&self) -> OperationSig<NonFunctionValue> {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![(
                "url".to_string(),
                NonFunctionValue::new(serde_json::json!({"type": "string"})),
            )],
            return_schema: Some(NonFunctionValue::new(serde_json::json!({"type": "string"}))),
        }
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        Err(Error::Operation {
            name: "fetch".to_string(),
            args: args.to_vec(),
        })
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
struct CachingFetch;

impl Operation<NonFunctionValue> for CachingFetch {
    fn signature(&self) -> OperationSig<NonFunctionValue> {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![(
                "url".to_string(),
                NonFunctionValue::new(serde_json::json!({"type": "string"})),
            )],
            return_schema: Some(NonFunctionValue::new(serde_json::json!({"type": "string"}))),
        }
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let url = args.first().and_then(|v| v.as_str()).unwrap_or("");
        if url == "cached" {
            Ok(NonFunctionValue::new("cached-result"))
        } else {
            Err(Error::Operation {
                name: "fetch".to_string(),
                args: args.to_vec(),
            })
        }
    }
}

// ── test hosts ───────────────────────────────────────────────────

#[derive(Clone)]
struct OpHost {
    add: Add,
    fetch: FetchOp,
}

impl Default for OpHost {
    fn default() -> Self {
        OpHost {
            add: Add,
            fetch: FetchOp,
        }
    }
}

impl Host<NonFunctionValue> for OpHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        match name {
            "fetch" => Some(&self.fetch),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct CachingHost {
    add: Add,
    fetch: CachingFetch,
}

impl Default for CachingHost {
    fn default() -> Self {
        CachingHost {
            add: Add,
            fetch: CachingFetch,
        }
    }
}

impl Host<NonFunctionValue> for CachingHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        match name {
            "fetch" => Some(&self.fetch),
            _ => None,
        }
    }
}

// ── tests: operation trait ───────────────────────────────────────

#[test]
fn operation_sig_has_name_and_params() {
    let op = FetchOp;
    let sig = op.signature();
    assert_eq!(sig.name, "fetch");
    assert_eq!(sig.params.len(), 1);
    assert_eq!(sig.params[0].0, "url");
}

#[test]
fn operation_perform_signals() {
    let op = FetchOp;
    let mut ctx = CanonicalContext::new();
    let result = op.perform(&[NonFunctionValue::new("http://example.com")], &mut ctx);
    assert!(matches!(result, Err(Error::Operation { ref name, .. }) if name == "fetch"));
}

// ── tests: VM dispatches operations ──────────────────────────────

#[test]
fn vm_eval_operation_signals_with_structured_data() {
    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call(
        "fetch",
        vec![Expr::value(NonFunctionValue::new("http://example.com"))],
    );
    let result = vm.eval(&expr, &mut ctx);

    match result {
        Err(Error::Operation { name, args }) => {
            assert_eq!(name, "fetch");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].as_str(), Some("http://example.com"));
        }
        other => panic!("expected Error::Operation, got {other:?}"),
    }
}

#[test]
fn vm_eval_operation_takes_precedence_over_function() {
    struct DualFetch;
    impl Function<NonFunctionValue> for DualFetch {
        fn call(
            &self,
            _args: &[NonFunctionValue],
            _ctx: &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
            Ok(NonFunctionValue::new("from-function"))
        }
    }
    impl Operation<NonFunctionValue> for DualFetch {
        fn signature(&self) -> OperationSig<NonFunctionValue> {
            OperationSig {
                name: "fetch".to_string(),
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
                name: "fetch".to_string(),
                args: vec![],
            })
        }
    }

    struct DualHost {
        fetch: DualFetch,
    }
    impl Host<NonFunctionValue> for DualHost {
        fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
            match name {
                "fetch" => Some(&self.fetch),
                _ => None,
            }
        }
        fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
            match name {
                "fetch" => Some(&self.fetch),
                _ => None,
            }
        }
    }

    let vm = crate::Vm::new(DualHost { fetch: DualFetch });
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call("fetch", vec![]);
    let result = vm.eval(&expr, &mut ctx);
    // Operation should take precedence
    assert!(matches!(result, Err(Error::Operation { .. })));
}

// ── tests: resumable eval with operations ────────────────────────

#[test]
fn resumable_eval_operation_yields_structured_value() {
    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]);
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Yielded {
            value,
            continuation,
        } => {
            // The yielded value should be structured operation data
            assert_eq!(
                value.get("type").and_then(|v| v.as_str()),
                Some("operation")
            );
            assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("fetch"));
            assert!(!continuation.is_empty());
        }
        other => panic!("expected Yielded, got {other:?}"),
    }
}

#[test]
fn resumable_eval_operation_resume_with_caching_host() {
    // add(fetch("data"), 10)
    let expr = Expr::call(
        "add",
        vec![
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
            Expr::value(NonFunctionValue::new(10)),
        ],
    );

    // Server: fetch operation signals
    let server_vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = server_vm.eval_resumable(&expr, &mut ctx, None);
    let (_yielded, continuation) = match result {
        EvalResult::Yielded {
            value,
            continuation,
        } => {
            assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("fetch"));
            (value, continuation)
        }
        other => panic!("expected Yielded, got {other:?}"),
    };

    // Serialize continuation
    let json = continuation.to_json();
    let restored = Continuation::from_json(&json).unwrap();

    // Client: resume with caching host — but "data" is not cached,
    // so it signals again. Use "cached" instead.
    let client_vm = crate::Vm::new(CachingHost::default());
    let result = client_vm.eval_resumable(&expr, &mut ctx, Some(restored));
    // "data" is not "cached", so it yields again
    match result {
        EvalResult::Yielded { value, .. } => {
            assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("fetch"));
        }
        other => panic!("expected Yielded, got {other:?}"),
    }
}

#[test]
fn resumable_eval_operation_completes_when_cached() {
    // fetch("cached") — CachingFetch returns Ok for "cached"
    let expr = Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("cached"))]);

    let vm = crate::Vm::new(CachingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Done(value) => {
            assert_eq!(value.as_str(), Some("cached-result"));
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn resumable_eval_operation_in_nested_call() {
    // add(fetch("cached"), fetch("data"))
    // First: fetch("cached") completes (CachingFetch returns Ok)
    // Then: fetch("data") signals
    let expr = Expr::call(
        "add",
        vec![
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("cached"))]),
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        ],
    );

    let vm = crate::Vm::new(CachingHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Yielded { value, .. } => {
            assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("fetch"));
        }
        other => panic!("expected Yielded for fetch(\"data\"), got {other:?}"),
    }
}
