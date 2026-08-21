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

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

// ── is_valid ─────────────────────────────────────────────────────

#[test]
fn is_valid_iso() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("2021-01-15T10:30:00Z")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_valid_epoch() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("1970-01-01T00:00:00Z")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_valid_far_future() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("9999-12-31T23:59:59Z")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_valid_invalid_string() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("not-a-date")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_valid_empty_string() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_valid_partial_date() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("2021-01-15")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_valid_invalid_month() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("2021-13-01T00:00:00Z")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_valid_invalid_day() {
    assert_eq!(
        call(&DateTimeModule, "is_valid", &[s("2021-02-30T00:00:00Z")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_valid_no_args_errors() {
    let result = call_err(&DateTimeModule, "is_valid", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_valid_non_string_errors() {
    let result = call_err(
        &DateTimeModule,
        "is_valid",
        &[NonFunctionValue::new(serde_json::json!(42))],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn is_valid_extra_args_ignored() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_valid",
            &[s("2021-01-15T10:30:00Z"), s("extra")]
        )
        .as_bool(),
        Some(true)
    );
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
fn vm_is_valid() {
    let expr = Expr::call(
        "datetime.is_valid",
        vec![Expr::value(s("2021-01-15T10:30:00Z"))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn vm_is_valid_false() {
    let expr = Expr::call("datetime.is_valid", vec![Expr::value(s("not-a-date"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.is_valid").is_some());
}
