use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = IntlModule.function("format_relative_time").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = IntlModule.function("format_relative_time").expect("function exists");
    let mut ctx = Context::new();
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
    let s = result.as_str().expect("string result");
    assert!(s.contains("5") && s.contains("second"));
}

#[test]
fn relative_time_past_seconds() {
    let result = call(&[n(-10.0), s("second")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("10") && s.contains("second"));
}

#[test]
fn relative_time_days() {
    let result = call(&[n(3.0), s("day")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("3") && s.contains("day"));
}

#[test]
fn relative_time_years() {
    let result = call(&[n(5.0), s("year")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("5") && s.contains("year"));
}

#[test]
fn relative_time_german() {
    let result = call(&[n(5.0), s("second"), s("de")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("5") && s.contains("Sekunde"));
}

#[test]
fn relative_time_short_style() {
    let result = call(&[n(5.0), s("second"), s("en"), obj(&[("style", "short")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("5") && s.contains("sec"));
}

#[test]
fn relative_time_narrow_style() {
    let result = call(&[n(5.0), s("second"), s("en"), obj(&[("style", "narrow")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("5"));
}

#[test]
fn relative_time_numeric_auto() {
    let result = call(&[n(0.0), s("day"), s("en"), obj(&[("numeric", "auto")])]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
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
    for unit in &["second", "minute", "hour", "day", "week", "month", "quarter", "year"] {
        let result = call(&[n(5.0), s(unit)]);
        assert!(result.as_str().is_some(), "unit {} should work", unit);
    }
}

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::Vm;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format_relative_time() {
    let expr = Expr::call("intl.format_relative_time", vec![Expr::value(n(5.0)), Expr::value(s("second"))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}
