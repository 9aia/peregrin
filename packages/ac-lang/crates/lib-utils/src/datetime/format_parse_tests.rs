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

// ── format ───────────────────────────────────────────────────────

#[test]
fn format_iso() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-01-15T10:30:00Z"), s("%Y-%m-%d")]
        )
        .as_str(),
        Some("2021-01-15")
    );
}

#[test]
fn format_time() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-01-15T10:30:00Z"), s("%H:%M:%S")]
        )
        .as_str(),
        Some("10:30:00")
    );
}

#[test]
fn format_full() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-01-15T10:30:00Z"), s("%Y-%m-%d %H:%M:%S")]
        )
        .as_str(),
        Some("2021-01-15 10:30:00")
    );
}

#[test]
fn format_month_name() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-03-15T10:30:00Z"), s("%B")]
        )
        .as_str(),
        Some("March")
    );
}

#[test]
fn format_short_month() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-03-15T10:30:00Z"), s("%b")]
        )
        .as_str(),
        Some("Mar")
    );
}

#[test]
fn format_weekday() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-01-15T10:30:00Z"), s("%A")]
        )
        .as_str(),
        Some("Friday")
    );
}

#[test]
fn format_default_iso() {
    let result = call(&DateTimeModule, "format", &[s("2021-01-15T10:30:00Z")]);
    assert_eq!(result.as_str(), Some("2021-01-15T10:30:00Z"));
}

#[test]
fn format_no_args_errors() {
    let result = call_err(&DateTimeModule, "format", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn format_one_arg_defaults_iso() {
    let result = call(&DateTimeModule, "format", &[s("2021-01-15T10:30:00Z")]);
    assert_eq!(result.as_str(), Some("2021-01-15T10:30:00Z"));
}

#[test]
fn format_non_string_first_arg_errors() {
    let result = call_err(&DateTimeModule, "format", &[n(42.0), s("%Y")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn format_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "format", &[s("not-a-date"), s("%Y")]);
    assert!(result.is_err());
}

#[test]
fn format_extra_args_ignored() {
    assert_eq!(
        call(
            &DateTimeModule,
            "format",
            &[s("2021-01-15T10:30:00Z"), s("%Y"), n(1.0)]
        )
        .as_str(),
        Some("2021")
    );
}

// ── parse ────────────────────────────────────────────────────────

#[test]
fn parse_iso() {
    let result = call(&DateTimeModule, "parse", &[s("2021-01-15T10:30:00Z")]);
    assert_eq!(result.as_str(), Some("2021-01-15T10:30:00Z"));
}

#[test]
fn parse_custom_format() {
    let result = call(
        &DateTimeModule,
        "parse",
        &[s("2021-01-15 10:30:00"), s("%Y-%m-%d %H:%M:%S")],
    );
    assert_eq!(result.as_str(), Some("2021-01-15T10:30:00Z"));
}

#[test]
fn parse_us_date() {
    let result = call(&DateTimeModule, "parse", &[s("01/15/2021"), s("%m/%d/%Y")]);
    assert_eq!(result.as_str(), Some("2021-01-15T00:00:00Z"));
}

#[test]
fn parse_eu_date() {
    let result = call(&DateTimeModule, "parse", &[s("15/01/2021"), s("%d/%m/%Y")]);
    assert_eq!(result.as_str(), Some("2021-01-15T00:00:00Z"));
}

#[test]
fn parse_no_args_errors() {
    let result = call_err(&DateTimeModule, "parse", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn parse_non_string_errors() {
    let result = call_err(&DateTimeModule, "parse", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn parse_invalid_string_errors() {
    let result = call_err(&DateTimeModule, "parse", &[s("not-a-date")]);
    assert!(result.is_err());
}

#[test]
fn parse_format_mismatch_errors() {
    let result = call_err(&DateTimeModule, "parse", &[s("2021-01-15"), s("%H:%M:%S")]);
    assert!(result.is_err());
}

#[test]
fn parse_extra_args_ignored() {
    let result = call(
        &DateTimeModule,
        "parse",
        &[s("2021-01-15T10:30:00Z"), s("%Y-%m-%dT%H:%M:%SZ"), n(1.0)],
    );
    assert_eq!(result.as_str(), Some("2021-01-15T10:30:00Z"));
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::Vm;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(DateTimeModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format() {
    let expr = Expr::call(
        "datetime.format",
        vec![
            Expr::value(s("2021-01-15T10:30:00Z")),
            Expr::value(s("%Y-%m-%d")),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-01-15"));
}

#[test]
fn vm_parse() {
    let expr = Expr::call(
        "datetime.parse",
        vec![Expr::value(s("2021-01-15T10:30:00Z"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-01-15T10:30:00Z"));
}
