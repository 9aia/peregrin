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

// ── to_timezone ─────────────────────────────────────────────────

#[test]
fn to_timezone_positive_offset() {
    let result = call(
        &DateTimeModule,
        "to_timezone",
        &[s("2021-06-15T10:30:00Z"), s("+05:30")],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T16:00:00+05:30"));
}

#[test]
fn to_timezone_negative_offset() {
    let result = call(
        &DateTimeModule,
        "to_timezone",
        &[s("2021-06-15T10:30:00Z"), s("-03:00")],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T07:30:00-03:00"));
}

#[test]
fn to_timezone_zero_offset() {
    let result = call(
        &DateTimeModule,
        "to_timezone",
        &[s("2021-06-15T10:30:00Z"), s("Z")],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:00+00:00"));
}

#[test]
fn to_timezone_cross_day() {
    let result = call(
        &DateTimeModule,
        "to_timezone",
        &[s("2021-06-15T22:00:00Z"), s("+05:00")],
    );
    assert_eq!(result.as_str(), Some("2021-06-16T03:00:00+05:00"));
}

#[test]
fn to_timezone_invalid_offset_errors() {
    let result = call_err(
        &DateTimeModule,
        "to_timezone",
        &[s("2021-06-15T10:30:00Z"), s("invalid")],
    );
    assert!(result.is_err());
}

#[test]
fn to_timezone_invalid_date_errors() {
    let result = call_err(
        &DateTimeModule,
        "to_timezone",
        &[s("not-a-date"), s("+05:30")],
    );
    assert!(result.is_err());
}

#[test]
fn to_timezone_no_args_errors() {
    let result = call_err(&DateTimeModule, "to_timezone", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── to_utc ───────────────────────────────────────────────────────

#[test]
fn to_utc_from_offset() {
    let result = call(&DateTimeModule, "to_utc", &[s("2021-06-15T16:00:00+05:30")]);
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:00Z"));
}

#[test]
fn to_utc_from_negative_offset() {
    let result = call(&DateTimeModule, "to_utc", &[s("2021-06-15T07:30:00-03:00")]);
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:00Z"));
}

#[test]
fn to_utc_already_utc() {
    let result = call(&DateTimeModule, "to_utc", &[s("2021-06-15T10:30:00Z")]);
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:00Z"));
}

#[test]
fn to_utc_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "to_utc", &[s("not-a-date")]);
    assert!(result.is_err());
}

#[test]
fn to_utc_no_args_errors() {
    let result = call_err(&DateTimeModule, "to_utc", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::{CanonicalContext, Host, Vm};
use ac_runtime_modules::ModuleHost;

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(DateTimeModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_to_timezone() {
    let expr = Expr::call(
        "datetime.to_timezone",
        vec![
            Expr::value(s("2021-06-15T10:30:00Z")),
            Expr::value(s("+05:30")),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-06-15T16:00:00+05:30"));
}

#[test]
fn vm_to_utc() {
    let expr = Expr::call(
        "datetime.to_utc",
        vec![Expr::value(s("2021-06-15T16:00:00+05:30"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:00Z"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.to_timezone").is_some());
    assert!(host.function("datetime.to_utc").is_some());
}
