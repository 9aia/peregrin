use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::DateTimeModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

const DATE: &str = "2021-06-15T10:30:45Z";

// ── year ─────────────────────────────────────────────────────────

#[test]
fn year() {
    assert_eq!(
        call(&DateTimeModule, "year", &[s(DATE)]).as_number(),
        Some(2021.0)
    );
}

#[test]
fn year_no_args_errors() {
    let result = call_err(&DateTimeModule, "year", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn year_non_string_errors() {
    let result = call_err(&DateTimeModule, "year", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── month ────────────────────────────────────────────────────────

#[test]
fn month() {
    assert_eq!(
        call(&DateTimeModule, "month", &[s(DATE)]).as_number(),
        Some(6.0)
    );
}

#[test]
fn month_january() {
    assert_eq!(
        call(&DateTimeModule, "month", &[s("2021-01-15T00:00:00Z")]).as_number(),
        Some(1.0)
    );
}

#[test]
fn month_december() {
    assert_eq!(
        call(&DateTimeModule, "month", &[s("2021-12-15T00:00:00Z")]).as_number(),
        Some(12.0)
    );
}

// ── day ──────────────────────────────────────────────────────────

#[test]
fn day() {
    assert_eq!(
        call(&DateTimeModule, "day", &[s(DATE)]).as_number(),
        Some(15.0)
    );
}

#[test]
fn day_first() {
    assert_eq!(
        call(&DateTimeModule, "day", &[s("2021-01-01T00:00:00Z")]).as_number(),
        Some(1.0)
    );
}

#[test]
fn day_last() {
    assert_eq!(
        call(&DateTimeModule, "day", &[s("2021-01-31T00:00:00Z")]).as_number(),
        Some(31.0)
    );
}

// ── hour ─────────────────────────────────────────────────────────

#[test]
fn hour() {
    assert_eq!(
        call(&DateTimeModule, "hour", &[s(DATE)]).as_number(),
        Some(10.0)
    );
}

#[test]
fn hour_midnight() {
    assert_eq!(
        call(&DateTimeModule, "hour", &[s("2021-01-01T00:00:00Z")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn hour_max() {
    assert_eq!(
        call(&DateTimeModule, "hour", &[s("2021-01-01T23:59:59Z")]).as_number(),
        Some(23.0)
    );
}

// ── minute ───────────────────────────────────────────────────────

#[test]
fn minute() {
    assert_eq!(
        call(&DateTimeModule, "minute", &[s(DATE)]).as_number(),
        Some(30.0)
    );
}

#[test]
fn minute_zero() {
    assert_eq!(
        call(&DateTimeModule, "minute", &[s("2021-01-01T10:00:00Z")]).as_number(),
        Some(0.0)
    );
}

// ── second ───────────────────────────────────────────────────────

#[test]
fn second() {
    assert_eq!(
        call(&DateTimeModule, "second", &[s(DATE)]).as_number(),
        Some(45.0)
    );
}

#[test]
fn second_zero() {
    assert_eq!(
        call(&DateTimeModule, "second", &[s("2021-01-01T10:30:00Z")]).as_number(),
        Some(0.0)
    );
}

// ── weekday ──────────────────────────────────────────────────────

#[test]
fn weekday_monday() {
    assert_eq!(
        call(&DateTimeModule, "weekday", &[s("2021-06-14T00:00:00Z")]).as_number(),
        Some(1.0)
    );
}

#[test]
fn weekday_sunday() {
    assert_eq!(
        call(&DateTimeModule, "weekday", &[s("2021-06-13T00:00:00Z")]).as_number(),
        Some(7.0)
    );
}

#[test]
fn weekday_friday() {
    assert_eq!(
        call(&DateTimeModule, "weekday", &[s("2021-06-18T00:00:00Z")]).as_number(),
        Some(5.0)
    );
}

// ── day_of_year ──────────────────────────────────────────────────

#[test]
fn day_of_year_jan1() {
    assert_eq!(
        call(&DateTimeModule, "day_of_year", &[s("2021-01-01T00:00:00Z")]).as_number(),
        Some(1.0)
    );
}

#[test]
fn day_of_year_dec31() {
    assert_eq!(
        call(&DateTimeModule, "day_of_year", &[s("2021-12-31T00:00:00Z")]).as_number(),
        Some(365.0)
    );
}

#[test]
fn day_of_year_leap_year_dec31() {
    assert_eq!(
        call(&DateTimeModule, "day_of_year", &[s("2020-12-31T00:00:00Z")]).as_number(),
        Some(366.0)
    );
}

#[test]
fn day_of_year_feb29_leap() {
    assert_eq!(
        call(&DateTimeModule, "day_of_year", &[s("2020-02-29T00:00:00Z")]).as_number(),
        Some(60.0)
    );
}

// ── week_of_year ─────────────────────────────────────────────────

#[test]
fn week_of_year_jan1() {
    let result = call(
        &DateTimeModule,
        "week_of_year",
        &[s("2021-01-01T00:00:00Z")],
    );
    let w = result.as_number().unwrap();
    assert!(w == 1.0 || w == 53.0, "week 1 or 53, got {w}");
}

#[test]
fn week_of_year_mid_year() {
    let result = call(
        &DateTimeModule,
        "week_of_year",
        &[s("2021-06-15T00:00:00Z")],
    );
    let w = result.as_number().unwrap();
    assert!(w >= 23.0 && w <= 25.0, "week 23-25, got {w}");
}

// ── quarter ──────────────────────────────────────────────────────

#[test]
fn quarter_q1() {
    assert_eq!(
        call(&DateTimeModule, "quarter", &[s("2021-02-15T00:00:00Z")]).as_number(),
        Some(1.0)
    );
}

#[test]
fn quarter_q2() {
    assert_eq!(
        call(&DateTimeModule, "quarter", &[s(DATE)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn quarter_q3() {
    assert_eq!(
        call(&DateTimeModule, "quarter", &[s("2021-08-15T00:00:00Z")]).as_number(),
        Some(3.0)
    );
}

#[test]
fn quarter_q4() {
    assert_eq!(
        call(&DateTimeModule, "quarter", &[s("2021-11-15T00:00:00Z")]).as_number(),
        Some(4.0)
    );
}

#[test]
fn quarter_boundary_jan() {
    assert_eq!(
        call(&DateTimeModule, "quarter", &[s("2021-01-01T00:00:00Z")]).as_number(),
        Some(1.0)
    );
}

#[test]
fn quarter_boundary_apr() {
    assert_eq!(
        call(&DateTimeModule, "quarter", &[s("2021-04-01T00:00:00Z")]).as_number(),
        Some(2.0)
    );
}

// ── is_leap_year ─────────────────────────────────────────────────

#[test]
fn is_leap_year_true() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_leap_year",
            &[s("2020-01-01T00:00:00Z")]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn is_leap_year_false() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_leap_year",
            &[s("2021-01-01T00:00:00Z")]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_leap_year_century() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_leap_year",
            &[s("1900-01-01T00:00:00Z")]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_leap_year_century_2000() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_leap_year",
            &[s("2000-01-01T00:00:00Z")]
        )
        .as_bool(),
        Some(true)
    );
}

// ── is_weekend ───────────────────────────────────────────────────

#[test]
fn is_weekend_saturday() {
    assert_eq!(
        call(&DateTimeModule, "is_weekend", &[s("2021-06-12T00:00:00Z")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_weekend_sunday() {
    assert_eq!(
        call(&DateTimeModule, "is_weekend", &[s("2021-06-13T00:00:00Z")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_weekend_monday_false() {
    assert_eq!(
        call(&DateTimeModule, "is_weekend", &[s("2021-06-14T00:00:00Z")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_weekend_friday_false() {
    assert_eq!(
        call(&DateTimeModule, "is_weekend", &[s("2021-06-18T00:00:00Z")]).as_bool(),
        Some(false)
    );
}

// ── invalid date errors ──────────────────────────────────────────

#[test]
fn components_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "year", &[s("not-a-date")]);
    assert!(result.is_err());
}

#[test]
fn components_no_args_errors() {
    let result = call_err(&DateTimeModule, "month", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── extra args ignored ───────────────────────────────────────────

#[test]
fn components_extra_args_ignored() {
    assert_eq!(
        call(&DateTimeModule, "year", &[s(DATE), n(1.0)]).as_number(),
        Some(2021.0)
    );
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(DateTimeModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_year() {
    let expr = Expr::call("datetime.year", vec![Expr::value(s(DATE))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(2021.0));
}

#[test]
fn vm_is_leap_year() {
    let expr = Expr::call(
        "datetime.is_leap_year",
        vec![Expr::value(s("2020-01-01T00:00:00Z"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.year").is_some());
    assert!(host.function("datetime.month").is_some());
    assert!(host.function("datetime.day").is_some());
    assert!(host.function("datetime.hour").is_some());
    assert!(host.function("datetime.minute").is_some());
    assert!(host.function("datetime.second").is_some());
    assert!(host.function("datetime.weekday").is_some());
    assert!(host.function("datetime.day_of_year").is_some());
    assert!(host.function("datetime.week_of_year").is_some());
    assert!(host.function("datetime.quarter").is_some());
    assert!(host.function("datetime.is_leap_year").is_some());
    assert!(host.function("datetime.is_weekend").is_some());
}
