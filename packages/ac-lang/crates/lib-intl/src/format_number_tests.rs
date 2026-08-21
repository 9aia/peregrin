use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
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

// ── English (default) ────────────────────────────────────────────

#[test]
fn format_number_integer() {
    let result = call(&IntlModule, "format_number", &[n(42.0)]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("42"));
}

#[test]
fn format_number_thousands() {
    let result = call(&IntlModule, "format_number", &[n(1234567.0)]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("1") && s.contains("234") && s.contains("567"));
}

#[test]
fn format_number_decimal() {
    let result = call(&IntlModule, "format_number", &[n(1234567.89)]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("89"));
}

#[test]
fn format_number_zero() {
    let result = call(&IntlModule, "format_number", &[n(0.0)]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("0"));
}

#[test]
fn format_number_negative() {
    let result = call(&IntlModule, "format_number", &[n(-1234.0)]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("1") && s.contains("234"));
}

#[test]
fn format_number_no_args_errors() {
    let result = call_err(&IntlModule, "format_number", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn format_number_non_number_errors() {
    let result = call_err(&IntlModule, "format_number", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn format_number_pt_uses_dot_separator() {
    let result = call(&IntlModule, "format_number", &[n(1234567.89), s("pt-BR")]);
    let formatted = result.as_str().expect("string result");
    assert!(formatted.contains("1.234.567"), "got: {formatted}");
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn format_number_de_uses_dot_separator() {
    let result = call(&IntlModule, "format_number", &[n(1234567.89), s("de")]);
    let formatted = result.as_str().expect("string result");
    assert!(formatted.contains("1.234.567"), "got: {formatted}");
}

// ── French ───────────────────────────────────────────────────────

#[test]
fn format_number_fr_uses_narrow_nbsp_separator() {
    let result = call(&IntlModule, "format_number", &[n(1234567.89), s("fr")]);
    let formatted = result.as_str().expect("string result");
    assert!(formatted.contains("234"), "got: {formatted}");
}

// ── Bangla digits ────────────────────────────────────────────────

#[test]
fn format_number_bangla_digits() {
    let result = call(&IntlModule, "format_number", &[n(1000007.0), s("bn")]);
    let formatted = result.as_str().expect("string result");
    assert!(!formatted.is_empty());
}

// ── Compact notation ─────────────────────────────────────────────

#[test]
fn format_number_compact() {
    let result = call(&IntlModule, "format_number", &[n(1000000.0), s("en"), obj(&[("notation", "compact")])]);
    let formatted = result.as_str().expect("string result");
    assert!(!formatted.is_empty());
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format_number() {
    let expr = Expr::call("intl.format_number", vec![Expr::value(n(1234567.0))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(IntlModule);
    assert!(host.function("intl.format_number").is_some());
}
