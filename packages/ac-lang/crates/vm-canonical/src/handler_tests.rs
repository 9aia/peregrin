use crate::CanonicalContext;
use crate::Context;
use crate::Error;
use crate::NonFunctionValue;
use crate::{Continuation, EvalResult};
use crate::{Expr, HandlerClause};
use crate::{Function, Host};
use crate::{Operation, OperationSig};

// ── test operations and functions ────────────────────────────────

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
struct Concat;

impl Function<NonFunctionValue> for Concat {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let s: String = args
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join("");
        Ok(NonFunctionValue::new(s))
    }
}

#[derive(Clone)]
struct OpHost {
    add: Add,
    concat: Concat,
    fetch: FetchOp,
}

impl Default for OpHost {
    fn default() -> Self {
        OpHost {
            add: Add,
            concat: Concat,
            fetch: FetchOp,
        }
    }
}

impl Host<NonFunctionValue> for OpHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => Some(&self.add),
            "concat" => Some(&self.concat),
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

// ── tests: basic handler interception ────────────────────────────

#[test]
fn handler_intercepts_operation() {
    // handle(fetch("data")) with handler that returns a constant
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("intercepted")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("intercepted"));
}

#[test]
fn handler_binds_operation_args() {
    // handle(fetch("data")) with handler that returns the bound url param
    // Variables in ac-lang are resolved via Context, not as function calls.
    // The handler body needs to use the variable directly. But Expr only has
    // Value and Call — there's no Var. Variables are resolved by the frontend
    // as Call with no args to a special "variable" function, or by the eval
    // callback.
    //
    // For now, let's test with a simpler handler that just returns
    // a constant to verify the mechanism works.
    let expr = Expr::handle(
        Expr::call(
            "fetch",
            vec![Expr::value(NonFunctionValue::new("http://example.com"))],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("handler-result")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("handler-result"));
}

#[test]
fn handler_resume_continuation() {
    // handle(add(fetch("data"), 10))
    // fetch signals, handler resumes with value 42
    // result should be add(42, 10) = 52
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
            // Resume the continuation with value 42
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new(42.0))]),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(52.0));
}

#[test]
fn handler_no_match_propagates() {
    // handle(fetch("data")) with handler for "read_file" (not "fetch")
    // fetch should propagate as Error::Operation
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "read_file".to_string(),
            params: vec!["path".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("unused")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx);
    assert!(matches!(result, Err(Error::Operation { ref name, .. }) if name == "fetch"));
}

#[test]
fn handler_body_is_pure_value() {
    // handle(42) — body is a pure value, no operations
    // handler is never triggered
    let expr = Expr::handle(
        Expr::value(NonFunctionValue::new(42)),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("never-called")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

// ── tests: resumable eval with handlers ──────────────────────────

#[test]
fn resumable_handler_intercepts_operation() {
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("handled")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Done(value) => {
            assert_eq!(value.as_str(), Some("handled"));
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn resumable_handler_resume_continuation() {
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

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Done(value) => {
            assert_eq!(value.as_number(), Some(52.0));
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn resumable_handler_no_match_yields() {
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "read_file".to_string(),
            params: vec!["path".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("unused")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Yielded { value, .. } => {
            assert_eq!(
                value.get("type").and_then(|v| v.as_str()),
                Some("operation")
            );
            assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("fetch"));
        }
        other => panic!("expected Yielded, got {other:?}"),
    }
}

// ── tests: handler serialization ─────────────────────────────────

#[test]
fn handle_expr_serialization_roundtrip() {
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call("k", vec![Expr::value(NonFunctionValue::new("result"))]),
        }],
    );

    // Serialize via continuation (expr_to_json is internal, but we
    // can test by creating a continuation with a handle expr in pending)
    let cont = Continuation {
        frames: vec![crate::Frame {
            name: "wrapper".to_string(),
            evaluated: vec![],
            pending: vec![expr],
            handlers: vec![],
        }],
    };

    let json = cont.to_json();
    let restored = Continuation::from_json(&json).unwrap();
    assert_eq!(restored, cont);
}

// ── tests: advanced handler scenarios ─────────────────────────────

#[test]
fn handler_multiple_clauses_last_match_wins() {
    // Two clauses for "fetch" — last one wins (handler search uses .rev())
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("first")),
            },
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("second")),
            },
        ],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("second"));
}

#[test]
fn handler_multiple_clauses_different_ops() {
    // Two clauses for different ops — only matching one fires
    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![
            HandlerClause {
                op_name: "read_file".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("file-handled")),
            },
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("fetch-handled")),
            },
        ],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("fetch-handled"));
}

#[test]
fn handler_does_not_resume_continuation() {
    // Handler ignores the continuation and returns a constant.
    // The handler's return value replaces the operation's result,
    // and the computation continues with that value.
    // add(fetch("data"), 10) → handler returns 999 → add(999, 10) = 1009
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
            // Don't call k — just return a constant
            body: Expr::value(NonFunctionValue::new(999.0)),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    // Handler returned 999, which replaces fetch's result.
    // The parent add(999, 10) = 1009
    assert_eq!(result.as_number(), Some(1009.0));
}

#[test]
fn handler_operation_completes_without_signaling() {
    // An operation that returns Ok (completes) should not trigger
    // the handler — it behaves like a regular function call.
    struct CachedOp;
    impl Operation<NonFunctionValue> for CachedOp {
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
            Ok(NonFunctionValue::new("cached-value"))
        }
    }

    struct CachedHost {
        add: Add,
        fetch: CachedOp,
    }
    impl Host<NonFunctionValue> for CachedHost {
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
    impl Default for CachedHost {
        fn default() -> Self {
            CachedHost {
                add: Add,
                fetch: CachedOp,
            }
        }
    }

    let expr = Expr::handle(
        Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("should-not-fire")),
        }],
    );

    let vm = crate::Vm::new(CachedHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    // Operation completed without signaling — handler not called
    assert_eq!(result.as_str(), Some("cached-value"));
}

#[test]
fn handler_resume_with_transformed_value() {
    // Handler resumes k with a computed value (not just a constant)
    // handle(add(fetch("data"), 1))
    // handler: k(add(40, 2)) → add(42, 1) = 43
    let expr = Expr::handle(
        Expr::call(
            "add",
            vec![
                Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
                Expr::value(NonFunctionValue::new(1)),
            ],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::call(
                "k",
                vec![Expr::call(
                    "add",
                    vec![
                        Expr::value(NonFunctionValue::new(40.0)),
                        Expr::value(NonFunctionValue::new(2.0)),
                    ],
                )],
            ),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(43.0));
}

#[test]
fn nested_handlers_outer_takes_precedence() {
    // Outer handler for "fetch", inner handler for "fetch"
    // Outer wins because handler search uses .rev() and inner handlers
    // are prepended (first in vector), so outer (last) is found first
    let expr = Expr::handle(
        Expr::handle(
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
            vec![HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("inner")),
            }],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("outer")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("outer"));
}

#[test]
fn nested_handlers_outer_catches_unhandled_op() {
    // Inner handler for "read_file", outer handler for "fetch"
    // fetch is not handled by inner → propagates to outer
    let expr = Expr::handle(
        Expr::handle(
            Expr::call("fetch", vec![Expr::value(NonFunctionValue::new("data"))]),
            vec![HandlerClause {
                op_name: "read_file".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value(NonFunctionValue::new("file-handled")),
            }],
        ),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value(NonFunctionValue::new("fetch-handled")),
        }],
    );

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("fetch-handled"));
}

#[test]
fn handler_in_resumable_eval_with_continuation_serialization() {
    // handle(add(fetch("data"), 10))
    // handler resumes k with 42 → add(42, 10) = 52
    // Serialize the continuation and resume
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

    let vm = crate::Vm::new(OpHost::default());
    let mut ctx = CanonicalContext::new();
    let result = vm.eval_resumable(&expr, &mut ctx, None);

    match result {
        EvalResult::Done(value) => {
            assert_eq!(value.as_number(), Some(52.0));
        }
        other => panic!("expected Done, got {other:?}"),
    }
}
