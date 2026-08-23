//! Integration tests: VM + handlers + operations + store end-to-end.
//!
//! These tests exercise the full pipeline from expression construction
//! through VM evaluation with handlers, operations, content-addressed
//! storage, and continuation serialization.

use ac_ast_canonical::{Expr, HandlerClause};
use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_store::{DefinitionHash, DefinitionStore, EvalCache, NameRegistry};
use ac_vm::{
    Context, Continuation, Error, EvalResult, Function, Host, Operation, OperationSig, Vm,
};

// ── test infrastructure ───────────────────────────────────────────

#[derive(Clone)]
struct Add;
impl Function for Add {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(sum))
    }
}

#[derive(Clone)]
struct Mul;
impl Function for Mul {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let product: f64 = args.iter().filter_map(|v| v.as_number()).product();
        Ok(NonFunctionValue::new(product))
    }
}

#[derive(Clone)]
struct Concat;
impl Function for Concat {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let s: String = args
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join("");
        Ok(NonFunctionValue::new(s))
    }
}

#[derive(Clone)]
struct FetchOp;
impl Operation for FetchOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "fetch".to_string(),
            params: vec![("url".to_string(), serde_json::json!({"type": "string"}))],
            return_schema: Some(serde_json::json!({"type": "string"})),
        }
    }
    fn perform(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "fetch".to_string(),
            args: args.to_vec(),
        })
    }
}

#[derive(Clone)]
struct LogOp;
impl Operation for LogOp {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "log".to_string(),
            params: vec![("msg".to_string(), serde_json::json!({"type": "string"}))],
            return_schema: None,
        }
    }
    fn perform(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "log".to_string(),
            args: vec![],
        })
    }
}

#[derive(Clone)]
struct FullHost {
    add: Add,
    mul: Mul,
    concat: Concat,
    fetch: FetchOp,
    log: LogOp,
}

impl Default for FullHost {
    fn default() -> Self {
        FullHost {
            add: Add,
            mul: Mul,
            concat: Concat,
            fetch: FetchOp,
            log: LogOp,
        }
    }
}

impl Host for FullHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            "concat" => Some(&self.concat),
            _ => None,
        }
    }
    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        match name {
            "fetch" => Some(&self.fetch),
            "log" => Some(&self.log),
            _ => None,
        }
    }
}

// ── tests: VM + handlers + operations ─────────────────────────────

#[test]
fn full_pipeline_handler_intercepts_fetch_and_resumes() {
    // handle(add(fetch("data"), 10))
    // handler resumes k with 42 → add(42, 10) = 52
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                Expr::value(NonFunctionValue::new(10)),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
        }],
    );

    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(52.0));
}

#[test]
fn full_pipeline_multi_op_handler() {
    // handle(add(fetch("data"), mul(log("msg"), 2)))
    // fetch handler: k(42) → 42
    // log handler: k(1) → 1
    // result: add(42, mul(1, 2)) = add(42, 2) = 44
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                Expr::call(
                    "mul",
                    vec![
                        Expr::call("log", vec![Expr::value(NonFunctionValue::new("msg"))]),
                        Expr::value(NonFunctionValue::new(2)),
                    ],
                ),
            ],
        ),
        vec![
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec!["url".to_string()],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
            },
            HandlerClause {
                op_name: "log".to_string(),
                params: vec!["msg".to_string()],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(1.0))]),
            },
        ],
    );

    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(44.0));
}

#[test]
fn full_pipeline_handler_returns_constant_without_resume() {
    // handle(fetch("data")) — handler returns "cached" without resuming
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("cached-result")),
        }],
    );

    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("cached-result"));
}

#[test]
fn full_pipeline_nested_handlers_different_ops() {
    // outer handle: catches "log"
    // inner handle: catches "fetch"
    // body: add(fetch("data"), log("msg"))
    let expr = Expr::handle(
        Expr::handle(
            Expr::call(
                "add",
                vec![
                    Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                    Expr::call("log", vec![Expr::value(NonFunctionValue::new("msg"))]),
                ],
            ),
            vec![HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(10.0))]),
            }],
        ),
        vec![HandlerClause {
            op_name: "log".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(5.0))]),
        }],
    );

    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    // fetch handled by inner → 10, log handled by outer → 5
    // add(10, 5) = 15
    assert_eq!(result.as_number(), Some(15.0));
}

// ── tests: continuation serialization across hosts ────────────────

#[test]
fn continuation_serialize_deserialize_resume() {
    // add(fetch("data"), 10) — fetch signals, we serialize the
    // continuation, deserialize it, and resume with a caching host.
    let expr = Expr::call(
        "add",
        vec![
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
            Expr::value(NonFunctionValue::new(10)),
        ],
    );

    // Start eval — fetch signals
    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    let (_yielded, cont) = match result {
        EvalResult::Yielded {
            value,
            continuation,
        } => (value, continuation),
        other => panic!("expected Yielded, got {other:?}"),
    };

    // Serialize continuation to JSON
    let json = cont.to_json();
    let restored = Continuation::from_json(&json).unwrap();
    assert_eq!(restored, cont);

    // Resume with a host that has a caching fetch
    #[derive(Clone)]
    struct CachingFetch;
    impl Operation for CachingFetch {
        fn signature(&self) -> OperationSig {
            OperationSig {
                name: "fetch".to_string(),
                params: vec![],
                return_schema: None,
            }
        }
        fn perform(
            &self,
            _args: &[NonFunctionValue],
            _ctx: &mut Context,
        ) -> Result<NonFunctionValue, Error> {
            Ok(NonFunctionValue::new(42.0))
        }
    }

    #[derive(Clone)]
    struct CachingHost {
        add: Add,
        fetch: CachingFetch,
    }
    impl Host for CachingHost {
        fn function(&self, name: &str) -> Option<&dyn Function> {
            match name {
                "add" => Some(&self.add),
                _ => None,
            }
        }
        fn operation(&self, name: &str) -> Option<&dyn Operation> {
            match name {
                "fetch" => Some(&self.fetch),
                _ => None,
            }
        }
    }
    impl Default for CachingHost {
        fn default() -> Self {
            CachingHost {
                add: Add,
                fetch: CachingFetch,
            }
        }
    }

    let mut client_vm = Vm::new(CachingHost::default());
    let result = client_vm.eval_resumable(&expr, &mut ctx, Some(restored));
    match result {
        EvalResult::Done(v) => assert_eq!(v.as_number(), Some(52.0)),
        other => panic!("expected Done, got {other:?}"),
    }
}

// ── tests: VM + content-addressed store ───────────────────────────

#[test]
fn store_and_eval_integration() {
    // Store a function definition, retrieve it by hash, and verify
    // its body matches what we stored.
    let mut store = DefinitionStore::new();
    let mut registry = NameRegistry::new();

    let double = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: Some(serde_json::json!({"type": "number"})),
        body: "mul(x, 2)".to_string(),
    };

    let hash = store.insert(double.clone());
    registry.register("double", hash.clone());

    // Resolve name → hash → definition
    let resolved_hash = registry.resolve("double").unwrap();
    let resolved_func = store.get(resolved_hash).unwrap();
    assert_eq!(resolved_func.body, "mul(x, 2)");

    // Verify the hash is a valid 64-char hex string
    let hex = hash.to_string();
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn eval_cache_integration() {
    // Create an eval cache, store a result, serialize/deserialize,
    // and verify the cached result is retrievable.
    let mut cache = EvalCache::new();
    let hash = DefinitionHash::from_hex("a".repeat(64));
    let args = vec![NonFunctionValue::new(21), NonFunctionValue::new(2)];

    // Simulate evaluating "double(21)" = 42
    cache.insert(&hash, &args, NonFunctionValue::new(42.0));

    // Verify cache hit
    let result = cache.get(&hash, &args).unwrap();
    assert_eq!(result.as_number(), Some(42.0));

    // Serialize and restore
    let json = cache.to_json();
    let restored = EvalCache::from_json(&json);
    assert_eq!(restored.len(), 1);

    let restored_result = restored.get(&hash, &args).unwrap();
    assert_eq!(restored_result.as_number(), Some(42.0));
}

// ── tests: handler + resumable eval + store ───────────────────────

#[test]
fn handler_with_resumable_eval_and_continuation_roundtrip() {
    // handle(add(fetch("data"), 10))
    // handler resumes k with 42 → add(42, 10) = 52
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                Expr::value(NonFunctionValue::new(10)),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
        }],
    );

    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();

    // This should complete in one step (handler intercepts inline)
    let result = vm.eval_resumable(&expr, &mut ctx, None);
    match result {
        EvalResult::Done(v) => assert_eq!(v.as_number(), Some(52.0)),
        other => panic!("expected Done, got {other:?}"),
    }
}

// ── tests: string operations with handlers ────────────────────────

#[test]
fn string_pipeline_with_handler() {
    // handle(concat(fetch("name"), "!"))
    // handler resumes k with "World" → concat("World", "!") = "World!"
    let expr = Expr::handle(
        Expr::call(
            "concat",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("name"))]),
                Expr::value(NonFunctionValue::new("!")),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new("World"))]),
        }],
    );

    let vm = Vm::new(FullHost::default());
    let mut ctx = Context::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("World!"));
}

// ── tests: codebase + store integration ───────────────────────────

#[test]
fn codebase_store_integration() {
    use ac_codebase::{Document, DocumentStore};

    let mut doc_store = DocumentStore::new();
    let mut def_store = DefinitionStore::new();
    let mut registry = NameRegistry::new();

    // Create and store function definitions
    let base_func = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: None,
        body: "add(x, 1)".to_string(),
    };
    let base_hash = def_store.insert(base_func.clone());
    registry.register("increment", base_hash.clone());

    // Create a document that uses the function
    let doc = Document::new().title("math-lib").entry("inc", base_func);
    let doc_hash = doc_store.insert(doc);

    // Verify document is stored
    assert!(doc_store.contains(&doc_hash));
    let retrieved_doc = doc_store.get(&doc_hash).unwrap();
    assert_eq!(retrieved_doc.title, Some("math-lib".to_string()));

    // Verify definition is accessible via registry
    let resolved = registry.resolve("increment").unwrap();
    let func = def_store.get(resolved).unwrap();
    assert_eq!(func.body, "add(x, 1)");

    // Resolve imports (no imports in this doc)
    let resolved_imports = doc_store.resolve_imports(&doc_hash);
    assert_eq!(resolved_imports.len(), 1);
    assert_eq!(resolved_imports[0], doc_hash);
}
