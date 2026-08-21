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

// ── module metadata ──────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(DateTimeModule.name(), "datetime");
}

#[test]
fn unknown_function_returns_none() {
    assert!(DateTimeModule.function("nope").is_none());
}

// ── from_unix ────────────────────────────────────────────────────

#[test]
fn from_unix_epoch() {
    assert_eq!(
        call(&DateTimeModule, "from_unix", &[n(0.0)]).as_str(),
        Some("1970-01-01T00:00:00Z")
    );
}

#[test]
fn from_unix_known_timestamp() {
    assert_eq!(
        call(&DateTimeModule, "from_unix", &[n(1609459200.0)]).as_str(),
        Some("2021-01-01T00:00:00Z")
    );
}

#[test]
fn from_unix_with_fractional() {
    let result = call(&DateTimeModule, "from_unix", &[n(1609459200.5)]);
    let s = result.as_str().expect("string result");
    assert!(s.starts_with("2021-01-01T00:00:00"));
}

#[test]
fn from_unix_negative() {
    assert_eq!(
        call(&DateTimeModule, "from_unix", &[n(-1.0)]).as_str(),
        Some("1969-12-31T23:59:59Z")
    );
}

#[test]
fn from_unix_no_args_errors() {
    let result = call_err(&DateTimeModule, "from_unix", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn from_unix_non_number_errors() {
    let result = call_err(&DateTimeModule, "from_unix", &[s("hello")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── to_unix ──────────────────────────────────────────────────────

#[test]
fn to_unix_epoch() {
    assert_eq!(
        call(&DateTimeModule, "to_unix", &[s("1970-01-01T00:00:00Z")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_unix_known() {
    assert_eq!(
        call(&DateTimeModule, "to_unix", &[s("2021-01-01T00:00:00Z")]).as_number(),
        Some(1609459200.0)
    );
}

#[test]
fn to_unix_roundtrip() {
    let original = 1609459200.0_f64;
    let iso = call(&DateTimeModule, "from_unix", &[n(original)]);
    let back = call(&DateTimeModule, "to_unix", &[iso]);
    assert_eq!(back.as_number(), Some(original));
}

#[test]
fn to_unix_no_args_errors() {
    let result = call_err(&DateTimeModule, "to_unix", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn to_unix_non_string_errors() {
    let result = call_err(&DateTimeModule, "to_unix", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn to_unix_invalid_string_errors() {
    let result = call_err(&DateTimeModule, "to_unix", &[s("not-a-date")]);
    assert!(result.is_err());
}

// ── now ──────────────────────────────────────────────────────────

#[test]
fn now_returns_number() {
    let result = call(&DateTimeModule, "now", &[]);
    assert!(result.as_number().is_some());
    let ts = result.as_number().unwrap();
    // Should be a reasonable timestamp (after 2020, before 2100)
    assert!(ts > 1577836800.0);
    assert!(ts < 4102444800.0);
}

#[test]
fn now_no_args_required() {
    let result = call_err(&DateTimeModule, "now", &[n(1.0)]);
    // now takes zero args — extra args are ignored per convention
    assert!(result.is_ok());
}

// ── utc_now ──────────────────────────────────────────────────────

#[test]
fn utc_now_returns_iso_string() {
    let result = call(&DateTimeModule, "utc_now", &[]);
    let s = result.as_str().expect("string result");
    assert!(s.ends_with('Z'));
    assert!(s.len() >= 20);
}

#[test]
fn utc_now_roundtrip_to_unix() {
    let iso = call(&DateTimeModule, "utc_now", &[]);
    let ts = call(&DateTimeModule, "to_unix", &[iso]);
    assert!(ts.as_number().is_some());
    let ts_val = ts.as_number().unwrap();
    assert!(ts_val > 1577836800.0);
}

// ── extra args ignored ───────────────────────────────────────────

#[test]
fn from_unix_extra_args_ignored() {
    assert_eq!(
        call(&DateTimeModule, "from_unix", &[n(0.0), n(1.0)]).as_str(),
        Some("1970-01-01T00:00:00Z")
    );
}

#[test]
fn to_unix_extra_args_ignored() {
    assert_eq!(
        call(
            &DateTimeModule,
            "to_unix",
            &[s("1970-01-01T00:00:00Z"), n(1.0)]
        )
        .as_number(),
        Some(0.0)
    );
}

// ── VM integration tests ─────────────────────────────────────────

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
fn vm_from_unix() {
    let expr = Expr::call("datetime.from_unix", vec![Expr::value(n(0.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("1970-01-01T00:00:00Z"));
}

#[test]
fn vm_to_unix() {
    let expr = Expr::call(
        "datetime.to_unix",
        vec![Expr::value(s("1970-01-01T00:00:00Z"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn vm_now() {
    let expr = Expr::call("datetime.now", vec![]);
    let result = eval_expr(&expr);
    assert!(result.as_number().is_some());
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.now").is_some());
    assert!(host.function("datetime.utc_now").is_some());
    assert!(host.function("datetime.from_unix").is_some());
    assert!(host.function("datetime.to_unix").is_some());
    assert!(host.function("datetime.nonexistent").is_none());
}
