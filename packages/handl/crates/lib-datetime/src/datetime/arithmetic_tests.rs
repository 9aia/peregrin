use handl_vm_common::Module;
use handl_vm_values_json::Context;
use handl_vm_values_json::Error;
use handl_vm_values_json::NonFunctionValue;

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

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

const BASE: &str = "2021-06-15T10:30:00Z";

// ── add_seconds ──────────────────────────────────────────────────

#[test]
fn add_seconds_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_seconds", &[s(BASE), n(90.0)]).as_str(),
        Some("2021-06-15T10:31:30Z")
    );
}

#[test]
fn add_seconds_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_seconds", &[s(BASE), n(-30.0)]).as_str(),
        Some("2021-06-15T10:29:30Z")
    );
}

#[test]
fn add_seconds_zero() {
    assert_eq!(
        call(&DateTimeModule, "add_seconds", &[s(BASE), n(0.0)]).as_str(),
        Some(BASE)
    );
}

#[test]
fn add_seconds_cross_minute() {
    assert_eq!(
        call(&DateTimeModule, "add_seconds", &[s(BASE), n(120.0)]).as_str(),
        Some("2021-06-15T10:32:00Z")
    );
}

#[test]
fn add_seconds_no_args_errors() {
    let result = call_err(&DateTimeModule, "add_seconds", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn add_seconds_non_string_first_errors() {
    let result = call_err(&DateTimeModule, "add_seconds", &[n(1.0), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn add_seconds_non_number_second_errors() {
    let result = call_err(&DateTimeModule, "add_seconds", &[s(BASE), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn add_seconds_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "add_seconds", &[s("not-a-date"), n(1.0)]);
    assert!(result.is_err());
}

// ── add_minutes ──────────────────────────────────────────────────

#[test]
fn add_minutes_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_minutes", &[s(BASE), n(60.0)]).as_str(),
        Some("2021-06-15T11:30:00Z")
    );
}

#[test]
fn add_minutes_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_minutes", &[s(BASE), n(-30.0)]).as_str(),
        Some("2021-06-15T10:00:00Z")
    );
}

#[test]
fn add_minutes_cross_day() {
    assert_eq!(
        call(&DateTimeModule, "add_minutes", &[s(BASE), n(900.0)]).as_str(),
        Some("2021-06-16T01:30:00Z")
    );
}

// ── add_hours ────────────────────────────────────────────────────

#[test]
fn add_hours_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_hours", &[s(BASE), n(5.0)]).as_str(),
        Some("2021-06-15T15:30:00Z")
    );
}

#[test]
fn add_hours_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_hours", &[s(BASE), n(-10.0)]).as_str(),
        Some("2021-06-15T00:30:00Z")
    );
}

#[test]
fn add_hours_cross_day() {
    assert_eq!(
        call(&DateTimeModule, "add_hours", &[s(BASE), n(24.0)]).as_str(),
        Some("2021-06-16T10:30:00Z")
    );
}

// ── add_days ─────────────────────────────────────────────────────

#[test]
fn add_days_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_days", &[s(BASE), n(10.0)]).as_str(),
        Some("2021-06-25T10:30:00Z")
    );
}

#[test]
fn add_days_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_days", &[s(BASE), n(-15.0)]).as_str(),
        Some("2021-05-31T10:30:00Z")
    );
}

#[test]
fn add_days_cross_month() {
    assert_eq!(
        call(
            &DateTimeModule,
            "add_days",
            &[s("2021-01-31T00:00:00Z"), n(1.0)]
        )
        .as_str(),
        Some("2021-02-01T00:00:00Z")
    );
}

#[test]
fn add_days_cross_year() {
    assert_eq!(
        call(
            &DateTimeModule,
            "add_days",
            &[s("2021-12-31T00:00:00Z"), n(1.0)]
        )
        .as_str(),
        Some("2022-01-01T00:00:00Z")
    );
}

// ── add_weeks ────────────────────────────────────────────────────

#[test]
fn add_weeks_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_weeks", &[s(BASE), n(2.0)]).as_str(),
        Some("2021-06-29T10:30:00Z")
    );
}

#[test]
fn add_weeks_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_weeks", &[s(BASE), n(-1.0)]).as_str(),
        Some("2021-06-08T10:30:00Z")
    );
}

// ── add_months ───────────────────────────────────────────────────

#[test]
fn add_months_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_months", &[s(BASE), n(3.0)]).as_str(),
        Some("2021-09-15T10:30:00Z")
    );
}

#[test]
fn add_months_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_months", &[s(BASE), n(-6.0)]).as_str(),
        Some("2020-12-15T10:30:00Z")
    );
}

#[test]
fn add_months_cross_year() {
    assert_eq!(
        call(
            &DateTimeModule,
            "add_months",
            &[s("2021-11-15T00:00:00Z"), n(3.0)]
        )
        .as_str(),
        Some("2022-02-15T00:00:00Z")
    );
}

#[test]
fn add_months_clamp_day() {
    // Jan 31 + 1 month = Feb 28 (clamped, not Mar 3)
    assert_eq!(
        call(
            &DateTimeModule,
            "add_months",
            &[s("2021-01-31T00:00:00Z"), n(1.0)]
        )
        .as_str(),
        Some("2021-02-28T00:00:00Z")
    );
}

#[test]
fn add_months_clamp_day_leap() {
    assert_eq!(
        call(
            &DateTimeModule,
            "add_months",
            &[s("2020-01-31T00:00:00Z"), n(1.0)]
        )
        .as_str(),
        Some("2020-02-29T00:00:00Z")
    );
}

// ── add_years ────────────────────────────────────────────────────

#[test]
fn add_years_basic() {
    assert_eq!(
        call(&DateTimeModule, "add_years", &[s(BASE), n(5.0)]).as_str(),
        Some("2026-06-15T10:30:00Z")
    );
}

#[test]
fn add_years_negative() {
    assert_eq!(
        call(&DateTimeModule, "add_years", &[s(BASE), n(-1.0)]).as_str(),
        Some("2020-06-15T10:30:00Z")
    );
}

#[test]
fn add_years_zero() {
    assert_eq!(
        call(&DateTimeModule, "add_years", &[s(BASE), n(0.0)]).as_str(),
        Some(BASE)
    );
}

#[test]
fn add_years_leap_day() {
    assert_eq!(
        call(
            &DateTimeModule,
            "add_years",
            &[s("2020-02-29T00:00:00Z"), n(1.0)]
        )
        .as_str(),
        Some("2021-02-28T00:00:00Z")
    );
}

#[test]
fn add_years_leap_day_to_leap() {
    assert_eq!(
        call(
            &DateTimeModule,
            "add_years",
            &[s("2020-02-29T00:00:00Z"), n(4.0)]
        )
        .as_str(),
        Some("2024-02-29T00:00:00Z")
    );
}

// ── arg count / type errors ──────────────────────────────────────

#[test]
fn add_days_no_args_errors() {
    let result = call_err(&DateTimeModule, "add_days", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn add_months_no_args_errors() {
    let result = call_err(&DateTimeModule, "add_months", &[s(BASE)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn add_years_non_number_errors() {
    let result = call_err(&DateTimeModule, "add_years", &[s(BASE), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── extra args ignored ───────────────────────────────────────────

#[test]
fn add_days_extra_args_ignored() {
    assert_eq!(
        call(&DateTimeModule, "add_days", &[s(BASE), n(1.0), n(2.0)]).as_str(),
        Some("2021-06-16T10:30:00Z")
    );
}

// ── VM integration ───────────────────────────────────────────────

use handl_ast_canonical::Expr;
use handl_vm_backend_rust_interpreter::{CanonicalContext, Host, Vm};
use handl_vm_common::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(DateTimeModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_add_days() {
    let expr = Expr::call(
        "datetime.add_days",
        vec![Expr::value(s(BASE)), Expr::value(n(10.0))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-06-25T10:30:00Z"));
}

#[test]
fn vm_add_months() {
    let expr = Expr::call(
        "datetime.add_months",
        vec![Expr::value(s(BASE)), Expr::value(n(3.0))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-09-15T10:30:00Z"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.add_seconds").is_some());
    assert!(host.function("datetime.add_minutes").is_some());
    assert!(host.function("datetime.add_hours").is_some());
    assert!(host.function("datetime.add_days").is_some());
    assert!(host.function("datetime.add_weeks").is_some());
    assert!(host.function("datetime.add_months").is_some());
    assert!(host.function("datetime.add_years").is_some());
}
