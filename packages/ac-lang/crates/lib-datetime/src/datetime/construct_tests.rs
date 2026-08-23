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

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

// ── from_components ──────────────────────────────────────────────

#[test]
fn from_components_date_only() {
    let result = call(
        &DateTimeModule,
        "from_components",
        &[n(2021.0), n(6.0), n(15.0)],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T00:00:00Z"));
}

#[test]
fn from_components_with_time() {
    let result = call(
        &DateTimeModule,
        "from_components",
        &[n(2021.0), n(6.0), n(15.0), n(10.0), n(30.0), n(45.0)],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:45Z"));
}

#[test]
fn from_components_partial_time() {
    let result = call(
        &DateTimeModule,
        "from_components",
        &[n(2021.0), n(6.0), n(15.0), n(10.0)],
    );
    assert_eq!(result.as_str(), Some("2021-06-15T10:00:00Z"));
}

#[test]
fn from_components_january_first() {
    let result = call(
        &DateTimeModule,
        "from_components",
        &[n(2000.0), n(1.0), n(1.0)],
    );
    assert_eq!(result.as_str(), Some("2000-01-01T00:00:00Z"));
}

#[test]
fn from_components_december_last() {
    let result = call(
        &DateTimeModule,
        "from_components",
        &[n(1999.0), n(12.0), n(31.0), n(23.0), n(59.0), n(59.0)],
    );
    assert_eq!(result.as_str(), Some("1999-12-31T23:59:59Z"));
}

#[test]
fn from_components_invalid_month_errors() {
    let result = call_err(
        &DateTimeModule,
        "from_components",
        &[n(2021.0), n(13.0), n(1.0)],
    );
    assert!(result.is_err());
}

#[test]
fn from_components_invalid_day_errors() {
    let result = call_err(
        &DateTimeModule,
        "from_components",
        &[n(2021.0), n(2.0), n(30.0)],
    );
    assert!(result.is_err());
}

#[test]
fn from_components_no_args_errors() {
    let result = call_err(&DateTimeModule, "from_components", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn from_components_two_args_errors() {
    let result = call_err(
        &DateTimeModule,
        "from_components",
        &[n(2021.0), n(6.0)],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn from_components_non_number_errors() {
    let result = call_err(
        &DateTimeModule,
        "from_components",
        &[s("hello"), n(6.0), n(15.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
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
fn vm_from_components() {
    let expr = Expr::call(
        "datetime.from_components",
        vec![
            Expr::value(n(2021.0)),
            Expr::value(n(6.0)),
            Expr::value(n(15.0)),
            Expr::value(n(10.0)),
            Expr::value(n(30.0)),
            Expr::value(n(45.0)),
        ],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("2021-06-15T10:30:45Z"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.from_components").is_some());
}
