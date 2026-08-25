use handl_vm_common::Module;
use handl_vm_values_json::{CanonicalContext, Error, FunctionValue, NonFunctionValue};
use serde_json::json;

use crate::test_helpers::{ErrorEval, TrueEval};
use crate::{
    AssertModule, BenchmarkModule, FuzzModule, PropertyModule, SnapshotModule, TestModule,
};

fn ctx() -> CanonicalContext {
    let mut context = CanonicalContext::new();
    context.set_eval(Box::new(TrueEval));
    context
}

fn err_ctx() -> CanonicalContext {
    let mut context = CanonicalContext::new();
    context.set_eval(Box::new(ErrorEval));
    context
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn fn_value(body: &str, params: &[&str]) -> NonFunctionValue {
    let params: Vec<_> = params
        .iter()
        .map(|p| (p.to_string(), serde_json::Value::Null))
        .collect();
    let value = FunctionValue {
        params,
        return_schema: None,
        body: body.to_string(),
        patterns: Vec::new(),
    };
    NonFunctionValue::new(value.to_json())
}

fn call(
    module: &dyn handl_vm_common::Module,
    name: &str,
    args: &[NonFunctionValue],
) -> NonFunctionValue {
    module
        .function(name)
        .expect("function exists")
        .call(args, &mut ctx())
        .unwrap_or_else(|e| NonFunctionValue::new(e.to_string()))
}

fn call_err(
    module: &dyn handl_vm_common::Module,
    name: &str,
    args: &[NonFunctionValue],
    context: &mut CanonicalContext,
) -> Result<NonFunctionValue, Error> {
    module
        .function(name)
        .expect("function exists")
        .call(args, context)
}

#[test]
fn assert_equal_passes_for_same_value() {
    let result = call(&AssertModule, "equal", &[n(3.0), n(3.0)]);
    assert_eq!(result, b(true));
}

#[test]
fn assert_equal_fails_for_different_value() {
    assert!(call_err(&AssertModule, "equal", &[n(3.0), n(4.0)], &mut ctx()).is_err());
}

#[test]
fn assert_not_equal_passes_for_different_value() {
    let result = call(&AssertModule, "not_equal", &[n(3.0), n(4.0)]);
    assert_eq!(result, b(true));
}

#[test]
fn assert_true_passes_for_true() {
    let result = call(&AssertModule, "true", &[b(true)]);
    assert_eq!(result, b(true));
}

#[test]
fn assert_false_passes_for_false() {
    let result = call(&AssertModule, "false", &[b(false)]);
    assert_eq!(result, b(true));
}

#[test]
fn assert_contains_finds_substring() {
    let result = call(&AssertModule, "contains", &[s("hello world"), s("world")]);
    assert_eq!(result, b(true));
}

#[test]
fn assert_throws_passes_when_function_errors() {
    let func = fn_value("error", &[]);
    let result = call_err(&AssertModule, "throws", &[func], &mut err_ctx());
    assert_eq!(result.unwrap(), b(true));
}

#[test]
fn test_run_reports_passing_cases() {
    let suite = NonFunctionValue::new(json!([
        { "name": "one", "body": fn_value("true", &[]).into_inner() },
        { "name": "two", "body": fn_value("true", &[]).into_inner() }
    ]));
    let report = call(&TestModule, "run", &[suite]);
    assert_eq!(report.get("passed").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(report.get("failed").and_then(|v| v.as_u64()), Some(0));
}

#[test]
fn test_run_reports_failing_cases() {
    let suite = NonFunctionValue::new(json!([
        { "name": "bad", "body": fn_value("error", &[]).into_inner() }
    ]));
    let mut context = err_ctx();
    let report = TestModule
        .function("run")
        .unwrap()
        .call(&[suite], &mut context)
        .unwrap();
    assert_eq!(report.get("passed").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(report.get("failed").and_then(|v| v.as_u64()), Some(1));
}

#[test]
fn benchmark_run_returns_report() {
    let func = fn_value("true", &[]);
    let result = call(&BenchmarkModule, "run", &[func, n(10.0)]);
    assert_eq!(result.get("iterations").and_then(|v| v.as_u64()), Some(10));
    assert_eq!(result.get("result"), Some(&json!(true)));
}

#[test]
fn property_for_all_passes_for_tautology() {
    let func = fn_value("true", &[]);
    let result = call(&PropertyModule, "for_all", &[func, n(20.0), n(12345.0)]);
    assert_eq!(result, b(true));
}

#[test]
fn property_for_all_fails_when_function_errors() {
    let func = fn_value("error", &[]);
    assert!(
        call_err(
            &PropertyModule,
            "for_all",
            &[func, n(1.0), n(1.0)],
            &mut err_ctx()
        )
        .is_err()
    );
}

#[test]
fn fuzz_run_reports_no_failures_for_tautology() {
    let func = fn_value("true", &[]);
    let result = call(&FuzzModule, "run", &[func, n(5.0), n(12345.0)]);
    assert_eq!(result.get("failures").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(result.get("runs").and_then(|v| v.as_u64()), Some(5));
}

#[test]
fn snapshot_match_passes_for_equal_values() {
    let result = call(
        &SnapshotModule,
        "match",
        &[s("greeting"), s("hello"), s("hello")],
    );
    assert_eq!(result, b(true));
}

#[test]
fn snapshot_format_serializes_value() {
    let value = NonFunctionValue::new(json!({ "a": 1, "b": [2, 3] }));
    let result = call(&SnapshotModule, "format", &[value]);
    assert!(result.as_str().is_some());
}
