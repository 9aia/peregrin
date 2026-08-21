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
fn ordinal_first() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(1.0)]).as_str(),
        Some("1st")
    );
}

#[test]
fn ordinal_second() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(2.0)]).as_str(),
        Some("2nd")
    );
}

#[test]
fn ordinal_third() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(3.0)]).as_str(),
        Some("3rd")
    );
}

#[test]
fn ordinal_fourth() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(4.0)]).as_str(),
        Some("4th")
    );
}

#[test]
fn ordinal_eleventh() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(11.0)]).as_str(),
        Some("11th")
    );
}

#[test]
fn ordinal_twelfth() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(12.0)]).as_str(),
        Some("12th")
    );
}

#[test]
fn ordinal_thirteenth() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(13.0)]).as_str(),
        Some("13th")
    );
}

#[test]
fn ordinal_twenty_first() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(21.0)]).as_str(),
        Some("21st")
    );
}

#[test]
fn ordinal_one_hundred_first() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(101.0)]).as_str(),
        Some("101st")
    );
}

#[test]
fn ordinal_one_hundred_eleventh() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(111.0)]).as_str(),
        Some("111th")
    );
}

#[test]
fn ordinal_no_args_errors() {
    let result = call_err(&IntlModule, "ordinal", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn ordinal_pt() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(1.0), s("pt")]).as_str(),
        Some("1º")
    );
}

#[test]
fn ordinal_pt_second() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(2.0), s("pt")]).as_str(),
        Some("2º")
    );
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn ordinal_de() {
    assert_eq!(
        call(&IntlModule, "ordinal", &[n(1.0), s("de")]).as_str(),
        Some("1.")
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
fn vm_ordinal() {
    let expr = Expr::call("intl.ordinal", vec![Expr::value(n(2.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2nd"));
}
