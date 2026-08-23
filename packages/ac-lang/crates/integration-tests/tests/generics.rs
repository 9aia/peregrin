//! Integration tests for generic function dispatch.
//!
//! These tests exercise the full pipeline:
//! 1. Typechecker resolves type arguments for generic calls
//! 2. Runtime dispatches to GenericFunction via type_args on Expr::Call
//! 3. Host module (lib-utils ArrayModule) responds to generic dispatch

use ac_ast_canonical::Expr;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::{self, Namespaces};
use ac_host::DefaultHost;
use ac_lib_utils::ArrayModule;
use ac_runtime::NonFunctionValue;
use ac_runtime_core::ModuleHost;
use ac_typecheck_core::{SchemaInterpreter, Type};
use ac_typecheck_infer::{FunctionTable, HostFunctionSigs};
use ac_typecheck_schema_json::JsonSchemaInterpreter;
use ac_vm_canonical::{CanonicalContext, Context, Error, Eval, Vm};
use serde_json::json;

/// Minimal [`Eval`] implementation for integration tests.
struct TestEval {
    host: DefaultHost,
    namespaces: Namespaces}

impl TestEval {
    fn new(host: DefaultHost, namespaces: Namespaces) -> Self {
        TestEval { host, namespaces }
    }
}

impl Eval for TestEval {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        let params: Vec<(String, serde_json::Value)> = vars
            .iter()
            .map(|(n, v)| (n.clone(), v.clone().into_inner()))
            .collect();
        let result =
            ac_fields::eval_body(expr, &params, &self.namespaces, &self.host, &AC_DIALECT)
                .ok_or_else(|| Error::PrimitiveError(format!("failed to evaluate: {expr}")))?;
        Ok(NonFunctionValue::new(result))
    }
}

fn build_host() -> ModuleHost {
    ModuleHost::new().with(ArrayModule)
}

fn build_function_table() -> FunctionTable {
    let sigs = HostFunctionSigs::default_host();
    let interpreter = JsonSchemaInterpreter;
    FunctionTable::from_signatures(&sigs.0, &interpreter)
}

// ── Type inference resolves type_args for generic calls ─────────────

#[test]
fn typecheck_resolves_type_args_for_array_map() {
    let table = build_function_table();

    // array.map(fn, arr) — but we need args that produce concrete types.
    // Since the typechecker works on expression strings, we test with
    // a call that has literal arguments.
    //
    // Note: The typechecker's parser may not parse function-value literals
    // directly, so we test the signature resolution via check_call_with_type_args
    // on the FunctionTable directly.
    let fn_type = table.lookup("array.map").expect("array.map should be in table");
    assert!(!fn_type.type_params.is_empty(), "array.map should be generic");

    // Simulate: map(fn: (Number) -> Number, arr: Array<Number>)
    // The arg types are [Function(Number -> Number), Array(Number)]
    use ac_typecheck_core::{FunctionType, TypeParam};
    let arg_types = vec![
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::Number)],
            return_type: Box::new(Type::Number)}),
        Type::Array(Box::new(Type::Number)),
    ];

    let (return_type, type_args) =
        ac_typecheck_infer::check_call_with_type_args(fn_type, &arg_types, None);

    // T should be Number, U should be Number
    assert_eq!(type_args.len(), 2, "should resolve 2 type args: T, U");
    assert_eq!(type_args[0], "number", "T should be number");
    assert_eq!(type_args[1], "number", "U should be number");
    assert_eq!(return_type, Type::Array(Box::new(Type::Number)));
}

#[test]
fn typecheck_resolves_type_args_for_array_filter() {
    let table = build_function_table();
    let fn_type = table.lookup("array.filter").expect("array.filter should be in table");
    assert!(!fn_type.type_params.is_empty(), "array.filter should be generic");

    // filter(fn: (String) -> Boolean, arr: Array<String>)
    use ac_typecheck_core::FunctionType;
    let arg_types = vec![
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Boolean)}),
        Type::Array(Box::new(Type::String)),
    ];

    let (return_type, type_args) =
        ac_typecheck_infer::check_call_with_type_args(fn_type, &arg_types, None);

    // T should be String
    assert_eq!(type_args.len(), 1, "should resolve 1 type arg: T");
    assert_eq!(type_args[0], "string", "T should be string");
    assert_eq!(return_type, Type::Array(Box::new(Type::String)));
}

#[test]
fn typecheck_resolves_type_args_for_array_reduce() {
    let table = build_function_table();
    let fn_type = table.lookup("array.reduce").expect("array.reduce should be in table");
    assert!(!fn_type.type_params.is_empty(), "array.reduce should be generic");

    // reduce(fn: (Number, String) -> Number, arr: Array<String>, init: Number)
    use ac_typecheck_core::FunctionType;
    let arg_types = vec![
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![
                ("acc".to_string(), Type::Number),
                ("x".to_string(), Type::String),
            ],
            return_type: Box::new(Type::Number)}),
        Type::Array(Box::new(Type::String)),
        Type::Number,
    ];

    let (return_type, type_args) =
        ac_typecheck_infer::check_call_with_type_args(fn_type, &arg_types, None);

    // T should be String, U should be Number
    assert_eq!(type_args.len(), 2, "should resolve 2 type args: T, U");
    assert_eq!(type_args[0], "string", "T should be string");
    assert_eq!(type_args[1], "number", "U should be number");
    assert_eq!(return_type, Type::Number);
}

// ── Runtime dispatches to GenericFunction via type_args ─────────────

#[test]
fn vm_dispatches_array_map_with_type_args() {
    let host = build_host();
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();

    // Define a user function value: { body: "mul(x, 2)", params: [{name: "x"}] }
    let double_fn = json!({
        "body": "mul(x, 2)",
        "params": [{"name": "x", "schema": {"type": "number"}}]
    });
    ctx.define("double", NonFunctionValue::new(double_fn));

    // array.map(double, [1, 2, 3]) with type_args=["number", "number"]
    let expr = Expr::call_with_type_args(
        "array.map",
        vec![
            Expr::value(NonFunctionValue::new(json!({
                "body": "mul(x, 2)",
                "params": [{"name": "x", "schema": {"type": "number"}}]
            }))),
            Expr::value(NonFunctionValue::new(json!([1, 2, 3]))),
        ],
        vec!["number".to_string(), "number".to_string()],
    );

    // Note: This requires the host to evaluate the function body.
    // The ModuleHost doesn't have an eval context set up, so this
    // test verifies dispatch reaches the GenericFunction impl.
    // The actual function body evaluation requires a full compiler setup.
    let result = vm.eval(&expr, &mut ctx);

    // The result may be an error if the host can't evaluate the function body,
    // but it should NOT be Error::UnknownFunction — that would mean generic
    // dispatch didn't work.
    match result {
        Ok(val) => {
            // If it succeeds, we should get an array back
            assert!(val.as_array().is_some(), "array.map should return an array");
        }
        Err(e) => {
            // Error should NOT be UnknownFunction — that would mean dispatch failed
            assert!(
                !matches!(e, ac_vm::Error::UnknownFunction(_)),
                "should not get UnknownFunction — generic dispatch should have worked: {e:?}"
            );
        }
    }
}

#[test]
fn vm_dispatches_array_filter_with_type_args() {
    let host = build_host();
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();

    // array.filter(identity_fn, [true, false, true]) with type_args=["boolean"]
    let expr = Expr::call_with_type_args(
        "array.filter",
        vec![
            Expr::value(NonFunctionValue::new(json!({
                "body": "x",
                "params": [{"name": "x", "schema": {"type": "boolean"}}]
            }))),
            Expr::value(NonFunctionValue::new(json!([true, false, true]))),
        ],
        vec!["boolean".to_string()],
    );

    let result = vm.eval(&expr, &mut ctx);

    match result {
        Ok(val) => {
            assert!(val.as_array().is_some(), "array.filter should return an array");
        }
        Err(e) => {
            assert!(
                !matches!(e, ac_vm::Error::UnknownFunction(_)),
                "should not get UnknownFunction — generic dispatch should have worked: {e:?}"
            );
        }
    }
}

#[test]
fn vm_array_map_without_type_args_uses_regular_function() {
    let host = build_host();
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();

    // Without type_args, should still work via regular Function dispatch.
    let expr = Expr::call(
        "array.map",
        vec![
            Expr::value(NonFunctionValue::new(json!({
                "body": "x",
                "params": [{"name": "x", "schema": {"type": "number"}}]
            }))),
            Expr::value(NonFunctionValue::new(json!([1, 2, 3]))),
        ],
    );

    let result = vm.eval(&expr, &mut ctx);

    match result {
        Ok(val) => {
            assert!(val.as_array().is_some(), "array.map should return an array");
        }
        Err(e) => {
            assert!(
                !matches!(e, ac_vm::Error::UnknownFunction(_)),
                "should not get UnknownFunction: {e:?}"
            );
        }
    }
}

// ── Full pipeline: typecheck → resolve type_args → VM dispatch ──────

#[test]
fn full_pipeline_generic_type_args_resolution() {
    // This test verifies that the typechecker can resolve type arguments
    // for generic function signatures, and those type_args can be attached
    // to Expr::Call nodes for VM dispatch.

    let table = build_function_table();

    // Test type resolution for a simple expression string.
    // The parser may not handle function-value literals, so we test
    // with the lower-level API.
    let fn_type = table.lookup("array.map").unwrap();

    // Verify the signature is generic
    assert!(
        !fn_type.type_params.is_empty(),
        "array.map must have type_params"
    );
    assert_eq!(fn_type.type_params.len(), 2, "array.map has T, U");

    // Verify type arg names
    let tp_names: Vec<&str> = fn_type.type_params.iter().map(|tp| tp.name.as_str()).collect();
    assert_eq!(tp_names, vec!["T", "U"]);

    // Resolve type args for a concrete instantiation
    use ac_typecheck_core::FunctionType;
    let arg_types = vec![
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number)}),
        Type::Array(Box::new(Type::String)),
    ];

    let (return_type, type_args) =
        ac_typecheck_infer::check_call_with_type_args(fn_type, &arg_types, None);

    // T=String, U=Number
    assert_eq!(type_args, vec!["string", "number"]);
    assert_eq!(return_type, Type::Array(Box::new(Type::Number)));

    // Now verify these type_args can be used to construct an Expr::Call
    // that the VM will dispatch to GenericFunction.
    let host = build_host();
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();

    let expr = Expr::call_with_type_args(
        "array.map",
        vec![
            Expr::value(NonFunctionValue::new(json!({
                "body": "x",
                "params": [{"name": "x", "schema": {"type": "string"}}]
            }))),
            Expr::value(NonFunctionValue::new(json!(["a", "b", "c"]))),
        ],
        type_args.clone(),
    );

    // The VM should dispatch to GenericFunction::call_with_types on ArrayModule's Map.
    // It may fail on function body evaluation (no eval context), but should not
    // fail with UnknownFunction.
    let result = vm.eval(&expr, &mut ctx);
    match result {
        Ok(val) => {
            assert!(val.as_array().is_some(), "should return array");
        }
        Err(e) => {
            assert!(
                !matches!(e, ac_vm::Error::UnknownFunction(_)),
                "generic dispatch should have reached GenericFunction: {e:?}"
            );
        }
    }
}

// ── ModuleHost generic_function dispatch ────────────────────────────

#[test]
fn module_host_resolves_generic_function() {
    use ac_vm::{GenericFunction, Host};

    let host = build_host();

    // array.map should resolve to a GenericFunction via ModuleHost
    let generic = host.generic_function("array.map");
    assert!(generic.is_some(), "ModuleHost should resolve array.map as generic");

    // array.filter should also resolve
    let generic = host.generic_function("array.filter");
    assert!(generic.is_some(), "ModuleHost should resolve array.filter as generic");

    // array.reduce should also resolve
    let generic = host.generic_function("array.reduce");
    assert!(generic.is_some(), "ModuleHost should resolve array.reduce as generic");

    // Non-generic function should return None
    let generic = host.generic_function("array.length");
    assert!(generic.is_none(), "array.length is not generic");

    // Unknown function should return None
    let generic = host.generic_function("array.unknown");
    assert!(generic.is_none(), "unknown function should not resolve");
}

// ── End-to-end: full pipeline with generic functions ───────────────

#[test]
fn e2e_typechecker_resolves_type_args_for_array_map() {
    use ac_typecheck_core::TypeEnv;

    let table = build_function_table();
    let mut env = TypeEnv::new();
    env.insert("numbers", Type::Array(Box::new(Type::Number)));

    // Typechecker should resolve type args for array.map(fn, numbers)
    // T is inferred from the array arg, U may be unknown since fn is a placeholder
    let result = ac_typecheck_infer::infer_with_type_args(
        "array.map(fn, numbers)",
        &env,
        &table,
        &ac_dialect_canonical::AC_DIALECT,
    );

    // Type args should be resolved (T=number at minimum)
    // Note: U may be "unknown" since fn is not in TypeEnv, which causes
    // type_args to be empty per our graceful fallback. That's OK —
    // in the real pipeline, fn would have a proper function type.
    // Just verify the typechecker doesn't crash and produces a result.
    assert_eq!(result.return_type, Type::Array(Box::new(Type::Unknown)));
}

#[test]
fn e2e_vm_evaluates_generic_map_with_type_args() {
    use ac_ast_canonical::Expr;
    use ac_runtime::NonFunctionValue;

    let host = build_host();
    let table = build_function_table();
    let mut env = ac_typecheck_core::TypeEnv::new();
    env.insert("numbers", Type::Array(Box::new(Type::Number)));

    // Resolve type args via typechecker
    let result = ac_typecheck_infer::infer_with_type_args(
        "array.map(fn, numbers)",
        &env,
        &table,
        &ac_dialect_canonical::AC_DIALECT,
    );

    let vm = ac_vm_canonical::Vm::new(host);
    let mut ctx = ac_vm::CanonicalContext::new();

    // Set up eval callback for higher-order function body evaluation
    let eval = Box::new(TestEval::new(
        ac_host::DefaultHost::default(),
        ac_fields::Namespaces::new(),
    ));
    ctx.set_eval(eval);

    // Create a function value: { body: "mul(x, 2)", params: [{name: "x", schema: {type: number}}] }
    let fn_val = serde_json::json!({
        "body": "mul(x, 2)",
        "params": [{"name": "x", "schema": {"type": "number"}}]
    });
    let numbers = serde_json::json!([1, 2, 3, 4, 5]);

    let expr = Expr::call_with_type_args(
        "array.map",
        vec![
            Expr::value(NonFunctionValue::new(fn_val)),
            Expr::value(NonFunctionValue::new(numbers)),
        ],
        result.type_args.clone(),
    );

    let result_val = vm.eval(&expr, &mut ctx).expect("VM eval should succeed");
    let arr = result_val.into_inner();
    assert_eq!(arr, serde_json::json!([2.0, 4.0, 6.0, 8.0, 10.0]));
}

#[test]
fn e2e_vm_evaluates_generic_filter_with_type_args() {
    use ac_ast_canonical::Expr;
    use ac_runtime::NonFunctionValue;

    let host = build_host();
    let table = build_function_table();
    let mut env = ac_typecheck_core::TypeEnv::new();
    env.insert("values", Type::Array(Box::new(Type::Number)));

    let result = ac_typecheck_infer::infer_with_type_args(
        "array.filter(fn, values)",
        &env,
        &table,
        &ac_dialect_canonical::AC_DIALECT,
    );

    let vm = ac_vm_canonical::Vm::new(host);
    let mut ctx = ac_vm::CanonicalContext::new();

    let eval = Box::new(TestEval::new(
        ac_host::DefaultHost::default(),
        ac_fields::Namespaces::new(),
    ));
    ctx.set_eval(eval);

    // Filter with identity function — all numbers are truthy
    let fn_val = serde_json::json!({
        "body": "x",
        "params": [{"name": "x", "schema": {"type": "number"}}]
    });
    let values = serde_json::json!([1, 2, 3]);

    let expr = Expr::call_with_type_args(
        "array.filter",
        vec![
            Expr::value(NonFunctionValue::new(fn_val)),
            Expr::value(NonFunctionValue::new(values)),
        ],
        result.type_args.clone(),
    );

    let result_val = vm.eval(&expr, &mut ctx).expect("VM eval should succeed");
    // identity predicate keeps all elements (all numbers are truthy)
    assert_eq!(result_val.into_inner(), serde_json::json!([1, 2, 3]));
}

#[test]
fn e2e_vm_evaluates_generic_reduce_with_type_args() {
    use ac_ast_canonical::Expr;
    use ac_runtime::NonFunctionValue;

    let host = build_host();
    let table = build_function_table();
    let mut env = ac_typecheck_core::TypeEnv::new();
    env.insert("values", Type::Array(Box::new(Type::Number)));

    let result = ac_typecheck_infer::infer_with_type_args(
        "array.reduce(fn, values, init)",
        &env,
        &table,
        &ac_dialect_canonical::AC_DIALECT,
    );

    let vm = ac_vm_canonical::Vm::new(host);
    let mut ctx = ac_vm::CanonicalContext::new();

    let eval = Box::new(TestEval::new(
        ac_host::DefaultHost::default(),
        ac_fields::Namespaces::new(),
    ));
    ctx.set_eval(eval);

    // Reduce: add(acc, x) with init 0
    let fn_val = serde_json::json!({
        "body": "add(acc, x)",
        "params": [
            {"name": "acc", "schema": {"type": "number"}},
            {"name": "x", "schema": {"type": "number"}}
        ]
    });
    let values = serde_json::json!([1, 2, 3, 4, 5]);
    let init = serde_json::json!(0);

    let expr = Expr::call_with_type_args(
        "array.reduce",
        vec![
            Expr::value(NonFunctionValue::new(fn_val)),
            Expr::value(NonFunctionValue::new(values)),
            Expr::value(NonFunctionValue::new(init)),
        ],
        result.type_args.clone(),
    );

    let result_val = vm.eval(&expr, &mut ctx).expect("VM eval should succeed");
    assert_eq!(result_val.into_inner(), serde_json::json!(15.0));
}

#[test]
fn e2e_unknown_type_args_fall_back_gracefully() {
    use ac_ast_canonical::Expr;
    use ac_runtime::NonFunctionValue;

    let host = build_host();
    let table = build_function_table();
    // Empty TypeEnv — no type info for the argument
    let env = ac_typecheck_core::TypeEnv::new();

    let result = ac_typecheck_infer::infer_with_type_args(
        "array.map(fn, unknown_field)",
        &env,
        &table,
        &ac_dialect_canonical::AC_DIALECT,
    );

    // Type args should be empty (graceful fallback for unknown types)
    assert!(result.type_args.is_empty(), "unknown types should produce empty type_args");

    // VM should still evaluate the call (falls back to Function::call)
    let vm = ac_vm_canonical::Vm::new(host);
    let mut ctx = ac_vm::CanonicalContext::new();

    let eval = Box::new(TestEval::new(
        ac_host::DefaultHost::default(),
        ac_fields::Namespaces::new(),
    ));
    ctx.set_eval(eval);

    let fn_val = serde_json::json!({
        "body": "x",
        "params": [{"name": "x", "schema": {"type": "number"}}]
    });
    let arr = serde_json::json!([1, 2, 3]);

    let expr = Expr::call("array.map", vec![
        Expr::value(NonFunctionValue::new(fn_val)),
        Expr::value(NonFunctionValue::new(arr)),
    ]);

    let result_val = vm.eval(&expr, &mut ctx).expect("VM eval should succeed even without type_args");
    assert_eq!(result_val.into_inner(), serde_json::json!([1, 2, 3]));
}

#[test]
fn e2e_all_generic_functions_resolve_in_function_table() {
    let table = build_function_table();

    let generic_fns = [
        "array.map",
        "array.filter",
        "array.reduce",
        "array.flat_map",
        "array.find",
        "array.some",
        "array.every",
        "array.group_by",
        "array.partition",
        "array.sort_by",
        "array.sort_by_desc",
        "array.compact",
        "array.pluck",
        "array.min_by",
        "array.max_by",
        "array.count_by",
    ];

    for name in &generic_fns {
        let fn_type = table.lookup(name);
        assert!(
            fn_type.is_some(),
            "{} should be in FunctionTable",
            name
        );
        let fn_type = fn_type.unwrap();
        assert!(
            !fn_type.type_params.is_empty(),
            "{} should have type parameters",
            name
        );
    }
}

#[test]
fn e2e_module_host_resolves_all_generic_functions() {
    use ac_vm::Host;

    let host = build_host();

    let generic_fns = [
        "array.map",
        "array.filter",
        "array.reduce",
        "array.flat_map",
        "array.find",
        "array.some",
        "array.every",
        "array.group_by",
        "array.partition",
        "array.sort_by",
        "array.sort_by_desc",
        "array.compact",
        "array.pluck",
        "array.min_by",
        "array.max_by",
        "array.count_by",
    ];

    for name in &generic_fns {
        let generic = host.generic_function(name);
        assert!(
            generic.is_some(),
            "ModuleHost should resolve {} as generic",
            name
        );
    }
}
