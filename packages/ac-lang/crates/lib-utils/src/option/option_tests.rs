use ac_runtime::Eval;
use ac_runtime_modules::Module;
use ac_runtime_values::{CanonicalContext, Error, NonFunctionValue};
use serde_json::Value as JsonValue;

use super::OptionModule;

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn some(v: NonFunctionValue) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!({
        "_tag": "Some",
        "value": v.inner().clone(),
    }))
}

fn none() -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!({"_tag": "None"}))
}

fn fn_value(body: &str, params: &[&str]) -> NonFunctionValue {
    let params_json: Vec<JsonValue> = params
        .iter()
        .map(|p| serde_json::json!({"name": p, "schema": {}}))
        .collect();
    NonFunctionValue::new(serde_json::json!({
        "params": params_json,
        "body": body,
    }))
}

struct TestEval;
impl Eval<NonFunctionValue> for TestEval {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        let expr = expr.trim();
        for (name, val) in vars {
            if name == expr {
                return Ok(val.clone());
            }
        }
        if let Ok(num) = expr.parse::<f64>() {
            return Ok(NonFunctionValue::new(num));
        }
        if expr == "true" {
            return Ok(NonFunctionValue::new(true));
        }
        if expr == "false" {
            return Ok(NonFunctionValue::new(false));
        }
        if expr == "null" {
            return Ok(NonFunctionValue::new(JsonValue::Null));
        }
        if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
            return Ok(NonFunctionValue::new(expr[1..expr.len() - 1].to_string()));
        }
        Err(Error::UnknownVariable(expr.to_string()))
    }
}

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = OptionModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = OptionModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx)
}

// ── constructors ───────────────────────────────────────────────────

#[test]
fn some_wraps_value() {
    let result = call("some", &[n(42.0)]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Some", "value": 42.0})
    );
}

#[test]
fn none_returns_none() {
    let result = call("none", &[]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

#[test]
fn some_requires_arg() {
    assert!(matches!(call_err("some", &[]), Err(Error::ArgumentCount { .. })));
}

// ── predicates ─────────────────────────────────────────────────────

#[test]
fn is_some_true() {
    let result = call("is_some", &[some(n(1.0))]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_some_false_for_none() {
    let result = call("is_some", &[none()]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_some_false_for_non_option() {
    let result = call("is_some", &[n(1.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_none_true() {
    let result = call("is_none", &[none()]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_none_false_for_some() {
    let result = call("is_none", &[some(n(1.0))]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_none_false_for_non_option() {
    let result = call("is_none", &[n(1.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn predicates_require_one_arg() {
    assert!(matches!(call_err("is_some", &[]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("is_none", &[]), Err(Error::ArgumentCount { .. })));
}

// ── unwrap ─────────────────────────────────────────────────────────

#[test]
fn unwrap_some() {
    let result = call("unwrap", &[some(n(42.0))]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn unwrap_none_errors() {
    assert!(call_err("unwrap", &[none()]).is_err());
}

#[test]
fn unwrap_non_option_errors() {
    assert!(matches!(call_err("unwrap", &[n(1.0)]), Err(Error::TypeMismatch { .. })));
}

#[test]
fn unwrap_requires_arg() {
    assert!(matches!(call_err("unwrap", &[]), Err(Error::ArgumentCount { .. })));
}

// ── unwrap_or / unwrap_or_else ─────────────────────────────────────

#[test]
fn unwrap_or_some_returns_value() {
    let result = call("unwrap_or", &[some(n(42.0)), n(99.0)]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn unwrap_or_none_returns_default() {
    let result = call("unwrap_or", &[none(), n(99.0)]);
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn unwrap_or_requires_two_args() {
    assert!(matches!(call_err("unwrap_or", &[some(n(1.0))]), Err(Error::ArgumentCount { .. })));
}

#[test]
fn unwrap_or_else_some_returns_value() {
    let result = call("unwrap_or_else", &[some(n(42.0)), fn_value("99", &[])]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn unwrap_or_else_none_calls_function() {
    let result = call("unwrap_or_else", &[none(), fn_value("99", &[])]);
    assert_eq!(result.as_number(), Some(99.0));
}

// ── map / flat_map / and_then ──────────────────────────────────────

#[test]
fn map_some() {
    let result = call("map", &[some(n(5.0)), fn_value("42", &[])]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Some", "value": 42.0})
    );
}

#[test]
fn map_some_identity() {
    let result = call("map", &[some(n(5.0)), fn_value("x", &["x"])]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Some", "value": 5.0})
    );
}

#[test]
fn map_none() {
    let result = call("map", &[none(), fn_value("42", &[])]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

#[test]
fn flat_map_some() {
    let result = call("flat_map", &[some(n(5.0)), fn_value("x", &["x"])]);
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn flat_map_none() {
    let result = call("flat_map", &[none(), fn_value("x", &["x"])]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

#[test]
fn and_then_alias() {
    let result = call("and_then", &[some(n(5.0)), fn_value("x", &["x"])]);
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn map_requires_function() {
    assert!(matches!(call_err("map", &[some(n(1.0)), n(1.0)]), Err(Error::TypeMismatch { .. })));
}

// ── filter ─────────────────────────────────────────────────────────

#[test]
fn filter_truthy_keeps_some() {
    let result = call("filter", &[some(n(5.0)), fn_value("x", &["x"])]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Some", "value": 5.0})
    );
}

#[test]
fn filter_falsy_discards_some() {
    let result = call("filter", &[some(n(0.0)), fn_value("x", &["x"])]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

#[test]
fn filter_none() {
    let result = call("filter", &[none(), fn_value("x", &["x"])]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

// ── map_or ─────────────────────────────────────────────────────────

#[test]
fn map_or_some() {
    let result = call("map_or", &[some(n(5.0)), n(99.0), fn_value("42", &[])]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn map_or_none() {
    let result = call("map_or", &[none(), n(99.0), fn_value("42", &[])]);
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn map_or_requires_three_args() {
    assert!(matches!(
        call_err("map_or", &[some(n(1.0)), n(99.0)]),
        Err(Error::ArgumentCount { .. })
    ));
}

// ── or / or_else / and / xor ───────────────────────────────────────

#[test]
fn or_some_keeps_first() {
    let result = call("or", &[some(n(1.0)), some(n(2.0))]);
    assert_eq!(result.inner(), some(n(1.0)).inner());
}

#[test]
fn or_none_returns_second() {
    let result = call("or", &[none(), some(n(2.0))]);
    assert_eq!(result.inner(), some(n(2.0)).inner());
}

#[test]
fn or_else_some_keeps_first() {
    let result = call("or_else", &[some(n(1.0)), fn_value("99", &[])]);
    assert_eq!(result.inner(), some(n(1.0)).inner());
}

#[test]
fn or_else_none_calls_function() {
    let result = call("or_else", &[none(), fn_value("99", &[])]);
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn and_some_returns_second() {
    let result = call("and", &[some(n(1.0)), some(n(2.0))]);
    assert_eq!(result.inner(), some(n(2.0)).inner());
}

#[test]
fn and_none_returns_none() {
    let result = call("and", &[none(), some(n(2.0))]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

#[test]
fn xor_first_some() {
    let result = call("xor", &[some(n(1.0)), none()]);
    assert_eq!(result.inner(), some(n(1.0)).inner());
}

#[test]
fn xor_second_some() {
    let result = call("xor", &[none(), some(n(2.0))]);
    assert_eq!(result.inner(), some(n(2.0)).inner());
}

#[test]
fn xor_both_some() {
    let result = call("xor", &[some(n(1.0)), some(n(2.0))]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}

#[test]
fn xor_both_none() {
    let result = call("xor", &[none(), none()]);
    assert_eq!(result.inner(), &serde_json::json!({"_tag": "None"}));
}
