use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("format_date").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("format_date").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
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
fn format_date_basic() {
    let result = call(&[s("2024-05-08")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2024") && s.contains("8"));
}

#[test]
fn format_date_german() {
    let result = call(&[s("2024-05-08"), s("de")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2024") && s.contains("5") && s.contains("8"));
}

#[test]
fn format_date_short() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("style", "short")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("24") || s.contains("2024"));
}

#[test]
fn format_date_long() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("style", "long")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("May") || s.contains("2024"));
}

#[test]
fn format_datetime() {
    let result = call(&[s("2024-05-08T14:30:00"), s("en"), obj(&[("type", "datetime")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2024") && s.contains("30"));
}

#[test]
fn format_datetime_short() {
    let result = call(&[s("2024-05-08T14:30:00"), s("en"), obj(&[("type", "datetime"), ("style", "short")])]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

#[test]
fn format_date_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn format_date_invalid_format_errors() {
    assert!(call_err(&[s("not-a-date")]).is_err());
}

#[test]
fn format_date_invalid_date_errors() {
    assert!(call_err(&[s("2024-13-45")]).is_err());
}

#[test]
fn format_date_japanese_locale() {
    let result = call(&[s("2024-05-08"), s("ja")]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::Vm;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format_date() {
    let expr = Expr::call("intl.format_date", vec![Expr::value(s("2024-05-08"))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}

#[test]
fn format_date_time_only() {
    let result = call(&[s("14:30:00"), s("en"), obj(&[("type", "time")])]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2:30") || str.contains("14:30"));
}

#[test]
fn format_date_time_only_short() {
    let result = call(&[s("14:30:00"), s("en"), obj(&[("type", "time"), ("style", "short")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn format_date_datetime_with_timezone() {
    let result = call(&[s("2024-05-08T14:30:00+05:30"), s("en"), obj(&[("type", "datetime")])]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"));
}

#[test]
fn format_date_datetime_with_z_timezone() {
    let result = call(&[s("2024-05-08T14:30:00Z"), s("en"), obj(&[("type", "datetime")])]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"));
}

#[test]
fn format_date_datetime_with_fractional_seconds() {
    let result = call(&[s("2024-05-08T14:30:00.123456"), s("en"), obj(&[("type", "datetime")])]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"));
}

#[test]
fn format_date_negative_year() {
    let result = call(&[s("-0044-03-15"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}
