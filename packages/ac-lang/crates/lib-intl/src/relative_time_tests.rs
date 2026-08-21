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
fn rel_past_seconds() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-45.0)]).as_str(),
        Some("45 seconds ago")
    );
}

#[test]
fn rel_past_one_minute() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-60.0)]).as_str(),
        Some("1 minute ago")
    );
}

#[test]
fn rel_past_minutes() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-125.0)]).as_str(),
        Some("2 minutes ago")
    );
}

#[test]
fn rel_past_one_hour() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-3600.0)]).as_str(),
        Some("1 hour ago")
    );
}

#[test]
fn rel_past_hours() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-7200.0)]).as_str(),
        Some("2 hours ago")
    );
}

#[test]
fn rel_past_one_day() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-86400.0)]).as_str(),
        Some("1 day ago")
    );
}

#[test]
fn rel_past_days() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-172800.0)]).as_str(),
        Some("2 days ago")
    );
}

#[test]
fn rel_future_seconds() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(45.0)]).as_str(),
        Some("in 45 seconds")
    );
}

#[test]
fn rel_future_one_hour() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(3600.0)]).as_str(),
        Some("in 1 hour")
    );
}

#[test]
fn rel_future_days() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(172800.0)]).as_str(),
        Some("in 2 days")
    );
}

#[test]
fn rel_now() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(0.0)]).as_str(),
        Some("now")
    );
}

#[test]
fn rel_no_args_errors() {
    let result = call_err(&IntlModule, "relative_time", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn rel_pt_past_one_hour() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-3600.0), s("pt")]).as_str(),
        Some("há 1 hora")
    );
}

#[test]
fn rel_pt_past_hours() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-7200.0), s("pt")]).as_str(),
        Some("há 2 horas")
    );
}

#[test]
fn rel_pt_future_one_hour() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(3600.0), s("pt")]).as_str(),
        Some("em 1 hora")
    );
}

#[test]
fn rel_pt_now() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(0.0), s("pt")]).as_str(),
        Some("agora")
    );
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn rel_de_past_one_hour() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(-3600.0), s("de")]).as_str(),
        Some("vor 1 Stunde")
    );
}

#[test]
fn rel_de_future_hours() {
    assert_eq!(
        call(&IntlModule, "relative_time", &[n(7200.0), s("de")]).as_str(),
        Some("in 2 Stunden")
    );
}

// ── VM integration ───────────────────────────────────────────────

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
fn vm_relative_time() {
    let expr = Expr::call("intl.relative_time", vec![Expr::value(n(-3600.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("1 hour ago"));
}
