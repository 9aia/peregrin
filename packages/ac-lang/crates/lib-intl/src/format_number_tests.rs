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

// ── English ──────────────────────────────────────────────────────

#[test]
fn format_number_integer() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(42.0)]).as_str(),
        Some("42")
    );
}

#[test]
fn format_number_thousands() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(1234567.0)]).as_str(),
        Some("1,234,567")
    );
}

#[test]
fn format_number_decimal() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(1234567.89)]).as_str(),
        Some("1,234,567.89")
    );
}

#[test]
fn format_number_small_decimal() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(42.5)]).as_str(),
        Some("42.5")
    );
}

#[test]
fn format_number_negative() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(-1234567.89)]).as_str(),
        Some("-1,234,567.89")
    );
}

#[test]
fn format_number_zero() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(0.0)]).as_str(),
        Some("0")
    );
}

#[test]
fn format_number_no_args_errors() {
    let result = call_err(&IntlModule, "format_number", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn format_number_pt_thousands() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(1234567.89), s("pt")]).as_str(),
        Some("1.234.567,89")
    );
}

#[test]
fn format_number_pt_integer() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(42.0), s("pt")]).as_str(),
        Some("42")
    );
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn format_number_de_thousands() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(1234567.89), s("de")]).as_str(),
        Some("1.234.567,89")
    );
}

// ── French (space thousands separator) ───────────────────────────

#[test]
fn format_number_fr_thousands() {
    assert_eq!(
        call(&IntlModule, "format_number", &[n(1234567.89), s("fr")]).as_str(),
        Some("1 234 567,89")
    );
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
    let expr = Expr::call("intl.format_number", vec![Expr::value(n(1234567.89))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("1,234,567.89"));
}
