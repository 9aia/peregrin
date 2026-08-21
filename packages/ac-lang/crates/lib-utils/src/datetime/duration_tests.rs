use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::DateTimeModule;

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

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

// ── duration_between ─────────────────────────────────────────────

#[test]
fn duration_between_seconds() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-01T00:00:00Z"),
        s("2021-01-01T00:00:30Z"),
    ]);
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(30.0));
    assert_eq!(obj.get("minutes").and_then(|v| v.as_f64()), Some(0.5));
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(30.0 / 3600.0));
    assert_eq!(obj.get("days").and_then(|v| v.as_f64()), Some(30.0 / 86400.0));
}

#[test]
fn duration_between_minutes() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-01T00:00:00Z"),
        s("2021-01-01T00:30:00Z"),
    ]);
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(1800.0));
    assert_eq!(obj.get("minutes").and_then(|v| v.as_f64()), Some(30.0));
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(0.5));
}

#[test]
fn duration_between_hours() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-01T00:00:00Z"),
        s("2021-01-01T05:30:00Z"),
    ]);
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(5.5));
    assert_eq!(obj.get("minutes").and_then(|v| v.as_f64()), Some(330.0));
}

#[test]
fn duration_between_days() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-01T00:00:00Z"),
        s("2021-01-04T00:00:00Z"),
    ]);
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("days").and_then(|v| v.as_f64()), Some(3.0));
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(72.0));
}

#[test]
fn duration_between_negative() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-04T00:00:00Z"),
        s("2021-01-01T00:00:00Z"),
    ]);
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("days").and_then(|v| v.as_f64()), Some(-3.0));
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(-259200.0));
}

#[test]
fn duration_between_equal() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-01T00:00:00Z"),
        s("2021-01-01T00:00:00Z"),
    ]);
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(0.0));
    assert_eq!(obj.get("days").and_then(|v| v.as_f64()), Some(0.0));
}

#[test]
fn duration_between_no_args_errors() {
    let result = call_err(&DateTimeModule, "duration_between", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn duration_between_one_arg_errors() {
    let result = call_err(&DateTimeModule, "duration_between", &[s("2021-01-01T00:00:00Z")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn duration_between_non_string_errors() {
    let result = call_err(&DateTimeModule, "duration_between", &[n(1.0), s("2021-01-01T00:00:00Z")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn duration_between_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "duration_between", &[
        s("not-a-date"),
        s("2021-01-01T00:00:00Z"),
    ]);
    assert!(result.is_err());
}

// ── duration_seconds ─────────────────────────────────────────────

#[test]
fn duration_seconds_positive() {
    assert_eq!(
        call(&DateTimeModule, "duration_seconds", &[
            s("2021-01-01T00:00:00Z"),
            s("2021-01-01T00:01:00Z"),
        ]).as_number(),
        Some(60.0)
    );
}

#[test]
fn duration_seconds_negative() {
    assert_eq!(
        call(&DateTimeModule, "duration_seconds", &[
            s("2021-01-01T00:01:00Z"),
            s("2021-01-01T00:00:00Z"),
        ]).as_number(),
        Some(-60.0)
    );
}

#[test]
fn duration_seconds_zero() {
    assert_eq!(
        call(&DateTimeModule, "duration_seconds", &[
            s("2021-01-01T00:00:00Z"),
            s("2021-01-01T00:00:00Z"),
        ]).as_number(),
        Some(0.0)
    );
}

#[test]
fn duration_seconds_no_args_errors() {
    let result = call_err(&DateTimeModule, "duration_seconds", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── humanize_duration ────────────────────────────────────────────

#[test]
fn humanize_seconds() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(45.0)]).as_str(),
        Some("45 seconds")
    );
}

#[test]
fn humanize_one_second() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(1.0)]).as_str(),
        Some("1 second")
    );
}

#[test]
fn humanize_minutes() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(120.0)]).as_str(),
        Some("2 minutes")
    );
}

#[test]
fn humanize_one_minute() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(60.0)]).as_str(),
        Some("1 minute")
    );
}

#[test]
fn humanize_minutes_and_seconds() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(125.0)]).as_str(),
        Some("2 minutes 5 seconds")
    );
}

#[test]
fn humanize_hours() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(7200.0)]).as_str(),
        Some("2 hours")
    );
}

#[test]
fn humanize_one_hour() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(3600.0)]).as_str(),
        Some("1 hour")
    );
}

#[test]
fn humanize_hours_and_minutes() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(3900.0)]).as_str(),
        Some("1 hour 5 minutes")
    );
}

#[test]
fn humanize_days() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(172800.0)]).as_str(),
        Some("2 days")
    );
}

#[test]
fn humanize_one_day() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(86400.0)]).as_str(),
        Some("1 day")
    );
}

#[test]
fn humanize_days_and_hours() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(90000.0)]).as_str(),
        Some("1 day 1 hour")
    );
}

#[test]
fn humanize_zero() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(0.0)]).as_str(),
        Some("0 seconds")
    );
}

#[test]
fn humanize_negative() {
    assert_eq!(
        call(&DateTimeModule, "humanize_duration", &[n(-125.0)]).as_str(),
        Some("-2 minutes 5 seconds")
    );
}

#[test]
fn humanize_no_args_errors() {
    let result = call_err(&DateTimeModule, "humanize_duration", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn humanize_non_number_errors() {
    let result = call_err(&DateTimeModule, "humanize_duration", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── extra args ignored ───────────────────────────────────────────

#[test]
fn duration_between_extra_args_ignored() {
    let result = call(&DateTimeModule, "duration_between", &[
        s("2021-01-01T00:00:00Z"),
        s("2021-01-01T00:00:30Z"),
        n(1.0),
    ]);
    assert!(result.as_object().is_some());
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(DateTimeModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_duration_seconds() {
    let expr = Expr::call("datetime.duration_seconds", vec![
        Expr::value(s("2021-01-01T00:00:00Z")),
        Expr::value(s("2021-01-01T00:01:00Z")),
    ]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(60.0));
}

#[test]
fn vm_humanize_duration() {
    let expr = Expr::call("datetime.humanize_duration", vec![Expr::value(n(125.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2 minutes 5 seconds"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.duration_between").is_some());
    assert!(host.function("datetime.duration_seconds").is_some());
    assert!(host.function("datetime.humanize_duration").is_some());
}
