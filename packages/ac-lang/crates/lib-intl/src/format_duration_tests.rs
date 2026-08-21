use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = IntlModule.function("format_duration").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = IntlModule.function("format_duration").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn obj(pairs: &[(&str, u64)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

fn opts(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

#[test]
fn format_duration_seconds() {
    let result = call(&[obj(&[("seconds", 90)])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("90") && s.contains("second"));
}

#[test]
fn format_duration_hours_minutes() {
    let result = call(&[obj(&[("hours", 1), ("minutes", 30)])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("1") && s.contains("30"));
}

#[test]
fn format_duration_german() {
    let result = call(&[obj(&[("seconds", 90)]), s("de")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("90") && s.contains("Sekunde"));
}

#[test]
fn format_duration_short_style() {
    let result = call(&[obj(&[("hours", 2), ("minutes", 5)]), s("en"), opts(&[("style", "short")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2") && s.contains("5"));
}

#[test]
fn format_duration_narrow_style() {
    let result = call(&[obj(&[("hours", 2)]), s("en"), opts(&[("style", "narrow")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2"));
}

#[test]
fn format_duration_digital_style() {
    let result = call(&[obj(&[("hours", 1), ("minutes", 30)]), s("en"), opts(&[("style", "digital")])]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

#[test]
fn format_duration_empty() {
    let result = call(&[obj(&[])]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

#[test]
fn format_duration_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn format_duration_all_fields() {
    let result = call(&[obj(&[
        ("years", 1), ("months", 2), ("weeks", 3), ("days", 4),
        ("hours", 5), ("minutes", 6), ("seconds", 7),
    ])]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
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
fn vm_format_duration() {
    let expr = Expr::call("intl.format_duration", vec![Expr::value(obj(&[("seconds", 90)]))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}
