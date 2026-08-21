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
    let result = call(&IntlModule::new(), "format_number", &[n(42.0)]);
    assert_eq!(result.as_str(), Some("42"));
}

#[test]
fn format_number_thousands() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.0)]);
    assert_eq!(result.as_str(), Some("1,234,567"));
}

#[test]
fn format_number_decimal() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.89)]);
    assert_eq!(result.as_str(), Some("1,234,567.89"));
}

#[test]
fn format_number_zero() {
    let result = call(&IntlModule::new(), "format_number", &[n(0.0)]);
    assert_eq!(result.as_str(), Some("0"));
}

#[test]
fn format_number_negative() {
    let result = call(&IntlModule::new(), "format_number", &[n(-1234.0)]);
    assert_eq!(result.as_str(), Some("-1,234"));
}

#[test]
fn format_number_no_args_errors() {
    let result = call_err(&IntlModule::new(), "format_number", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn format_number_non_number_errors() {
    let result = call_err(&IntlModule::new(), "format_number", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn format_number_pt_uses_dot_separator() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.89), s("pt-BR")]);
    assert_eq!(result.as_str(), Some("1.234.567,89"));
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn format_number_de_uses_dot_separator() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.89), s("de")]);
    assert_eq!(result.as_str(), Some("1.234.567,89"));
}

// ── French ───────────────────────────────────────────────────────

#[test]
fn format_number_fr_uses_narrow_nbsp_separator() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.89), s("fr")]);
    assert_eq!(result.as_str(), Some("1\u{202f}234\u{202f}567,89"));
}

// ── Bangla digits ────────────────────────────────────────────────

#[test]
fn format_number_bangla_digits() {
    let result = call(&IntlModule::new(), "format_number", &[n(1000007.0), s("bn")]);
    assert_eq!(result.as_str(), Some("১০,০০,০০৭"));
}

// ── Compact notation ─────────────────────────────────────────────

#[test]
fn format_number_compact() {
    let result = call(&IntlModule::new(), "format_number", &[n(1000000.0), s("en"), obj(&[("notation", "compact")])]);
    let formatted = result.as_str().expect("string result");
    assert!(!formatted.is_empty());
}

// ── Percent style ────────────────────────────────────────────────

#[test]
fn format_number_percent_half() {
    let result = call(&IntlModule::new(), "format_number", &[n(0.5), s("en"), obj(&[("style", "percent")])]);
    assert_eq!(result.as_str(), Some("50%"));
}

#[test]
fn format_number_percent_whole() {
    let result = call(&IntlModule::new(), "format_number", &[n(1.0), s("en"), obj(&[("style", "percent")])]);
    assert_eq!(result.as_str(), Some("100%"));
}

#[test]
fn format_number_percent_decimal() {
    let result = call(&IntlModule::new(), "format_number", &[n(0.1234), s("en"), obj(&[("style", "percent")])]);
    assert_eq!(result.as_str(), Some("12.34%"));
}

#[test]
fn format_number_percent_german() {
    let result = call(&IntlModule::new(), "format_number", &[n(0.5), s("de"), obj(&[("style", "percent")])]);
    assert_eq!(result.as_str(), Some("50\u{a0}%"));
}

// ── Currency style ───────────────────────────────────────────────

#[test]
fn format_number_currency_usd_symbol() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234.56), s("en"), obj(&[("style", "currency"), ("currency", "USD")])]);
    assert_eq!(result.as_str(), Some("$1,234.56"));
}

#[test]
fn format_number_currency_usd_code() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234.56), s("en"), obj(&[("style", "currency"), ("currency", "USD"), ("currencyDisplay", "code")])]);
    assert_eq!(result.as_str(), Some("USD\u{a0}1,234.56"));
}

#[test]
fn format_number_currency_eur_german() {
    let result = call(&IntlModule::new(), "format_number", &[n(999.99), s("de"), obj(&[("style", "currency"), ("currency", "EUR")])]);
    assert_eq!(result.as_str(), Some("999,99\u{a0}€"));
}

#[test]
fn format_number_currency_jpy() {
    let result = call(&IntlModule::new(), "format_number", &[n(5000.0), s("ja"), obj(&[("style", "currency"), ("currency", "JPY")])]);
    assert_eq!(result.as_str(), Some("￥5,000"));
}

#[test]
fn format_number_currency_missing_code_errors() {
    let result = call_err(&IntlModule::new(), "format_number", &[n(100.0), s("en"), obj(&[("style", "currency")])]);
    assert!(result.is_err());
}

#[test]
fn format_number_currency_invalid_code_errors() {
    let result = call_err(&IntlModule::new(), "format_number", &[n(100.0), s("en"), obj(&[("style", "currency"), ("currency", "dollar")])]);
    assert!(result.is_err());
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
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
    let host = ModuleHost::new().with(IntlModule::new());
    assert!(host.function("intl.format_number").is_some());
}
