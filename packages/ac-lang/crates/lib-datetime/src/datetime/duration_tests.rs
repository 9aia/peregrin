use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{Error};
use ac_runtime_values::{Context};
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

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

// ── duration_between ─────────────────────────────────────────────

#[test]
fn duration_between_seconds() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:00:30Z")],
    );
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(30.0));
    assert_eq!(obj.get("minutes").and_then(|v| v.as_f64()), Some(0.5));
    assert_eq!(
        obj.get("hours").and_then(|v| v.as_f64()),
        Some(30.0 / 3600.0)
    );
    assert_eq!(
        obj.get("days").and_then(|v| v.as_f64()),
        Some(30.0 / 86400.0)
    );
}

#[test]
fn duration_between_minutes() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:30:00Z")],
    );
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(1800.0));
    assert_eq!(obj.get("minutes").and_then(|v| v.as_f64()), Some(30.0));
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(0.5));
}

#[test]
fn duration_between_hours() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z"), s("2021-01-01T05:30:00Z")],
    );
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(5.5));
    assert_eq!(obj.get("minutes").and_then(|v| v.as_f64()), Some(330.0));
}

#[test]
fn duration_between_days() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z"), s("2021-01-04T00:00:00Z")],
    );
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("days").and_then(|v| v.as_f64()), Some(3.0));
    assert_eq!(obj.get("hours").and_then(|v| v.as_f64()), Some(72.0));
}

#[test]
fn duration_between_negative() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-04T00:00:00Z"), s("2021-01-01T00:00:00Z")],
    );
    let obj = result.as_object().expect("object result");
    assert_eq!(obj.get("days").and_then(|v| v.as_f64()), Some(-3.0));
    assert_eq!(obj.get("seconds").and_then(|v| v.as_f64()), Some(-259200.0));
}

#[test]
fn duration_between_equal() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:00:00Z")],
    );
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
    let result = call_err(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z")],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn duration_between_non_string_errors() {
    let result = call_err(
        &DateTimeModule,
        "duration_between",
        &[n(1.0), s("2021-01-01T00:00:00Z")],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn duration_between_invalid_date_errors() {
    let result = call_err(
        &DateTimeModule,
        "duration_between",
        &[s("not-a-date"), s("2021-01-01T00:00:00Z")],
    );
    assert!(result.is_err());
}

// ── duration_seconds ─────────────────────────────────────────────

#[test]
fn duration_seconds_positive() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_seconds",
            &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:01:00Z"),]
        )
        .as_number(),
        Some(60.0)
    );
}

#[test]
fn duration_seconds_negative() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_seconds",
            &[s("2021-01-01T00:01:00Z"), s("2021-01-01T00:00:00Z"),]
        )
        .as_number(),
        Some(-60.0)
    );
}

#[test]
fn duration_seconds_zero() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_seconds",
            &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:00:00Z"),]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn duration_seconds_no_args_errors() {
    let result = call_err(&DateTimeModule, "duration_seconds", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── duration_minutes ─────────────────────────────────────────────

#[test]
fn duration_minutes_basic() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_minutes",
            &[s("2021-01-01T00:00:00Z"), s("2021-01-01T01:30:00Z")]
        )
        .as_number(),
        Some(90.0)
    );
}

#[test]
fn duration_minutes_negative() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_minutes",
            &[s("2021-01-01T01:30:00Z"), s("2021-01-01T00:00:00Z")]
        )
        .as_number(),
        Some(-90.0)
    );
}

#[test]
fn duration_minutes_zero() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_minutes",
            &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:00:00Z")]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn duration_minutes_no_args_errors() {
    let result = call_err(&DateTimeModule, "duration_minutes", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── duration_hours ───────────────────────────────────────────────

#[test]
fn duration_hours_basic() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_hours",
            &[s("2021-01-01T00:00:00Z"), s("2021-01-01T05:30:00Z")]
        )
        .as_number(),
        Some(5.5)
    );
}

#[test]
fn duration_hours_negative() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_hours",
            &[s("2021-01-01T05:30:00Z"), s("2021-01-01T00:00:00Z")]
        )
        .as_number(),
        Some(-5.5)
    );
}

#[test]
fn duration_hours_no_args_errors() {
    let result = call_err(&DateTimeModule, "duration_hours", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── duration_days ────────────────────────────────────────────────

#[test]
fn duration_days_basic() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_days",
            &[s("2021-01-01T00:00:00Z"), s("2021-01-04T12:00:00Z")]
        )
        .as_number(),
        Some(3.5)
    );
}

#[test]
fn duration_days_negative() {
    assert_eq!(
        call(
            &DateTimeModule,
            "duration_days",
            &[s("2021-01-04T12:00:00Z"), s("2021-01-01T00:00:00Z")]
        )
        .as_number(),
        Some(-3.5)
    );
}

#[test]
fn duration_days_no_args_errors() {
    let result = call_err(&DateTimeModule, "duration_days", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── extra args ignored ───────────────────────────────────────────

#[test]
fn duration_between_extra_args_ignored() {
    let result = call(
        &DateTimeModule,
        "duration_between",
        &[s("2021-01-01T00:00:00Z"), s("2021-01-01T00:00:30Z"), n(1.0)],
    );
    assert!(result.as_object().is_some());
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
fn vm_duration_seconds() {
    let expr = Expr::call(
        "datetime.duration_seconds",
        vec![
            Expr::value(s("2021-01-01T00:00:00Z")),
            Expr::value(s("2021-01-01T00:01:00Z")),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(60.0));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.duration_between").is_some());
    assert!(host.function("datetime.duration_seconds").is_some());
    assert!(host.function("datetime.duration_minutes").is_some());
    assert!(host.function("datetime.duration_hours").is_some());
    assert!(host.function("datetime.duration_days").is_some());
}