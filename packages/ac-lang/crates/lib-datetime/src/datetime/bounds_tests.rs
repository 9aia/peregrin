use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error};
use ac_runtime_modules::Module;

use super::DateTimeModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

// ── start_of_day / end_of_day ────────────────────────────────────

#[test]
fn start_of_day() {
    let result = call(
        &DateTimeModule,
        "start_of_day",
        &[s("2021-06-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T00:00:00Z"));
}

#[test]
fn end_of_day() {
    let result = call(&DateTimeModule, "end_of_day", &[s("2021-06-15T10:30:45Z")]);
    assert_eq!(result.as_str(), Some("2021-06-15T23:59:59Z"));
}

// ── start_of_hour / end_of_hour ──────────────────────────────────

#[test]
fn start_of_hour() {
    let result = call(
        &DateTimeModule,
        "start_of_hour",
        &[s("2021-06-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T10:00:00Z"));
}

#[test]
fn end_of_hour() {
    let result = call(&DateTimeModule, "end_of_hour", &[s("2021-06-15T10:30:45Z")]);
    assert_eq!(result.as_str(), Some("2021-06-15T10:59:59Z"));
}

// ── start_of_month / end_of_month ────────────────────────────────

#[test]
fn start_of_month() {
    let result = call(
        &DateTimeModule,
        "start_of_month",
        &[s("2021-06-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-06-01T00:00:00Z"));
}

#[test]
fn end_of_month() {
    let result = call(
        &DateTimeModule,
        "end_of_month",
        &[s("2021-06-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-06-30T23:59:59Z"));
}

#[test]
fn end_of_month_feb_leap() {
    let result = call(
        &DateTimeModule,
        "end_of_month",
        &[s("2020-02-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2020-02-29T23:59:59Z"));
}

#[test]
fn end_of_month_feb_non_leap() {
    let result = call(
        &DateTimeModule,
        "end_of_month",
        &[s("2021-02-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-02-28T23:59:59Z"));
}

#[test]
fn end_of_month_december() {
    let result = call(
        &DateTimeModule,
        "end_of_month",
        &[s("2021-12-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-12-31T23:59:59Z"));
}

// ── start_of_year / end_of_year ──────────────────────────────────

#[test]
fn start_of_year() {
    let result = call(
        &DateTimeModule,
        "start_of_year",
        &[s("2021-06-15T10:30:45Z")],
    );
    assert_eq!(result.as_str(), Some("2021-01-01T00:00:00Z"));
}

#[test]
fn end_of_year() {
    let result = call(&DateTimeModule, "end_of_year", &[s("2021-06-15T10:30:45Z")]);
    assert_eq!(result.as_str(), Some("2021-12-31T23:59:59Z"));
}

// ── error cases ──────────────────────────────────────────────────

#[test]
fn start_of_day_no_args_errors() {
    let result = call_err(&DateTimeModule, "start_of_day", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn end_of_month_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "end_of_month", &[s("not-a-date")]);
    assert!(result.is_err());
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::{CanonicalContext, Host, Vm};
use ac_runtime_modules::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(DateTimeModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_start_of_day() {
    let expr = Expr::call(
        "datetime.start_of_day",
        vec![Expr::value(s("2021-06-15T10:30:45Z"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-06-15T00:00:00Z"));
}

#[test]
fn vm_end_of_month() {
    let expr = Expr::call(
        "datetime.end_of_month",
        vec![Expr::value(s("2021-06-15T10:30:45Z"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-06-30T23:59:59Z"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.start_of_day").is_some());
    assert!(host.function("datetime.end_of_day").is_some());
    assert!(host.function("datetime.start_of_hour").is_some());
    assert!(host.function("datetime.end_of_hour").is_some());
    assert!(host.function("datetime.start_of_month").is_some());
    assert!(host.function("datetime.end_of_month").is_some());
    assert!(host.function("datetime.start_of_year").is_some());
    assert!(host.function("datetime.end_of_year").is_some());
}
