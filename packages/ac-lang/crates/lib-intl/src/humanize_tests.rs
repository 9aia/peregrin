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

// ── English (default) ────────────────────────────────────────────

#[test]
fn humanize_seconds() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(45.0)]).as_str(),
        Some("45 seconds")
    );
}

#[test]
fn humanize_one_second() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(1.0)]).as_str(),
        Some("1 second")
    );
}

#[test]
fn humanize_minutes() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(120.0)]).as_str(),
        Some("2 minutes")
    );
}

#[test]
fn humanize_one_minute() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(60.0)]).as_str(),
        Some("1 minute")
    );
}

#[test]
fn humanize_minutes_and_seconds() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(125.0)]).as_str(),
        Some("2 minutes 5 seconds")
    );
}

#[test]
fn humanize_hours() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(7200.0)]).as_str(),
        Some("2 hours")
    );
}

#[test]
fn humanize_one_hour() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(3600.0)]).as_str(),
        Some("1 hour")
    );
}

#[test]
fn humanize_hours_and_minutes() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(3900.0)]).as_str(),
        Some("1 hour 5 minutes")
    );
}

#[test]
fn humanize_days() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(172800.0)]).as_str(),
        Some("2 days")
    );
}

#[test]
fn humanize_one_day() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(86400.0)]).as_str(),
        Some("1 day")
    );
}

#[test]
fn humanize_days_and_hours() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(90000.0)]).as_str(),
        Some("1 day 1 hour")
    );
}

#[test]
fn humanize_zero() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(0.0)]).as_str(),
        Some("0 seconds")
    );
}

#[test]
fn humanize_negative() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(-125.0)]).as_str(),
        Some("-2 minutes 5 seconds")
    );
}

#[test]
fn humanize_no_args_errors() {
    let result = call_err(&IntlModule, "humanize_duration", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn humanize_non_number_errors() {
    let result = call_err(&IntlModule, "humanize_duration", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── Portuguese locale ────────────────────────────────────────────

#[test]
fn humanize_pt_seconds() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(45.0), s("pt")]).as_str(),
        Some("45 segundos")
    );
}

#[test]
fn humanize_pt_one_second() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(1.0), s("pt")]).as_str(),
        Some("1 segundo")
    );
}

#[test]
fn humanize_pt_minutes_and_seconds() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(125.0), s("pt")]).as_str(),
        Some("2 minutos 5 segundos")
    );
}

#[test]
fn humanize_pt_one_day() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(86400.0), s("pt")]).as_str(),
        Some("1 dia")
    );
}

#[test]
fn humanize_pt_days() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(172800.0), s("pt")]).as_str(),
        Some("2 dias")
    );
}

// ── German locale ────────────────────────────────────────────────

#[test]
fn humanize_de_one_hour() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(3600.0), s("de")]).as_str(),
        Some("1 Stunde")
    );
}

#[test]
fn humanize_de_hours() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(7200.0), s("de")]).as_str(),
        Some("2 Stunden")
    );
}

// ── Japanese locale (no plural distinction) ──────────────────────

#[test]
fn humanize_ja_one_day() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(86400.0), s("ja")]).as_str(),
        Some("1 日")
    );
}

#[test]
fn humanize_ja_days() {
    assert_eq!(
        call(&IntlModule, "humanize_duration", &[n(172800.0), s("ja")]).as_str(),
        Some("2 日")
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
fn vm_humanize_duration() {
    let expr = Expr::call("intl.humanize_duration", vec![Expr::value(n(125.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2 minutes 5 seconds"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(IntlModule);
    assert!(host.function("intl.humanize_duration").is_some());
}
