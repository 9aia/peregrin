use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{CanonicalContext, Error};
use ac_runtime_values::{Context};
use ac_runtime_modules::Module;

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module
        .function("format_relative_time")
        .expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module
        .function("format_relative_time")
        .expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn obj(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

#[test]
fn relative_time_future_seconds() {
    let result = call(&[n(5.0), s("second")]);
    assert_eq!(result.as_str(), Some("in 5 seconds"));
}

#[test]
fn relative_time_past_seconds() {
    let result = call(&[n(-10.0), s("second")]);
    assert_eq!(result.as_str(), Some("10 seconds ago"));
}

#[test]
fn relative_time_days() {
    let result = call(&[n(3.0), s("day")]);
    assert_eq!(result.as_str(), Some("in 3 days"));
}

#[test]
fn relative_time_years() {
    let result = call(&[n(5.0), s("year")]);
    assert_eq!(result.as_str(), Some("in 5 years"));
}

#[test]
fn relative_time_german() {
    let result = call(&[n(5.0), s("second"), s("de")]);
    assert_eq!(result.as_str(), Some("in 5 Sekunden"));
}

#[test]
fn relative_time_short_style() {
    let result = call(&[n(5.0), s("second"), s("en"), obj(&[("style", "short")])]);
    assert_eq!(result.as_str(), Some("in 5 sec."));
}

#[test]
fn relative_time_narrow_style() {
    let result = call(&[n(5.0), s("second"), s("en"), obj(&[("style", "narrow")])]);
    assert_eq!(result.as_str(), Some("in 5s"));
}

#[test]
fn relative_time_numeric_auto() {
    let result = call(&[n(0.0), s("day"), s("en"), obj(&[("numeric", "auto")])]);
    assert_eq!(result.as_str(), Some("today"));
}

#[test]
fn relative_time_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn relative_time_missing_unit_errors() {
    assert!(call_err(&[n(5.0)]).is_err());
}

#[test]
fn relative_time_invalid_unit_errors() {
    assert!(call_err(&[n(5.0), s("decade")]).is_err());
}

#[test]
fn relative_time_all_units() {
    for unit in &[
        "second", "minute", "hour", "day", "week", "month", "quarter", "year",
    ] {
        let result = call(&[n(5.0), s(unit)]);
        assert!(result.as_str().is_some(), "unit {} should work", unit);
    }
}

use ac_ast_canonical::Expr;
use ac_runtime_canonical::Vm;
use ac_runtime_modules::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format_relative_time() {
    let expr = Expr::call(
        "intl.format_relative_time",
        vec![Expr::value(n(5.0)), Expr::value(s("second"))],
    );
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}