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

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

const EARLY: &str = "2021-01-01T00:00:00Z";
const MID: &str = "2021-06-15T10:30:00Z";
const LATE: &str = "2021-12-31T23:59:59Z";

// ── compare ──────────────────────────────────────────────────────

#[test]
fn compare_before() {
    assert_eq!(
        call(&DateTimeModule, "compare", &[s(EARLY), s(LATE)]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn compare_after() {
    assert_eq!(
        call(&DateTimeModule, "compare", &[s(LATE), s(EARLY)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn compare_equal() {
    assert_eq!(
        call(&DateTimeModule, "compare", &[s(MID), s(MID)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn compare_no_args_errors() {
    let result = call_err(&DateTimeModule, "compare", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn compare_one_arg_errors() {
    let result = call_err(&DateTimeModule, "compare", &[s(EARLY)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn compare_non_string_errors() {
    let result = call_err(&DateTimeModule, "compare", &[n(1.0), s(EARLY)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn compare_invalid_date_errors() {
    let result = call_err(&DateTimeModule, "compare", &[s("not-a-date"), s(EARLY)]);
    assert!(result.is_err());
}

// ── is_before ────────────────────────────────────────────────────

#[test]
fn is_before_true() {
    assert_eq!(
        call(&DateTimeModule, "is_before", &[s(EARLY), s(LATE)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_before_false_equal() {
    assert_eq!(
        call(&DateTimeModule, "is_before", &[s(MID), s(MID)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_before_false_after() {
    assert_eq!(
        call(&DateTimeModule, "is_before", &[s(LATE), s(EARLY)]).as_bool(),
        Some(false)
    );
}

// ── is_after ─────────────────────────────────────────────────────

#[test]
fn is_after_true() {
    assert_eq!(
        call(&DateTimeModule, "is_after", &[s(LATE), s(EARLY)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_after_false_equal() {
    assert_eq!(
        call(&DateTimeModule, "is_after", &[s(MID), s(MID)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_after_false_before() {
    assert_eq!(
        call(&DateTimeModule, "is_after", &[s(EARLY), s(LATE)]).as_bool(),
        Some(false)
    );
}

// ── is_equal ─────────────────────────────────────────────────────

#[test]
fn is_equal_true() {
    assert_eq!(
        call(&DateTimeModule, "is_equal", &[s(MID), s(MID)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_equal_false() {
    assert_eq!(
        call(&DateTimeModule, "is_equal", &[s(EARLY), s(LATE)]).as_bool(),
        Some(false)
    );
}

// ── is_between ───────────────────────────────────────────────────

#[test]
fn is_between_true() {
    assert_eq!(
        call(&DateTimeModule, "is_between", &[s(MID), s(EARLY), s(LATE)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_between_false_before() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between",
            &[s("2020-01-01T00:00:00Z"), s(EARLY), s(LATE)]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_between_false_after() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between",
            &[s("2022-01-01T00:00:00Z"), s(EARLY), s(LATE)]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_between_false_equal_start() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between",
            &[s(EARLY), s(EARLY), s(LATE)]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_between_false_equal_end() {
    assert_eq!(
        call(&DateTimeModule, "is_between", &[s(LATE), s(EARLY), s(LATE)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_between_no_args_errors() {
    let result = call_err(&DateTimeModule, "is_between", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_between_two_args_errors() {
    let result = call_err(&DateTimeModule, "is_between", &[s(MID), s(EARLY)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── extra args ignored ───────────────────────────────────────────

#[test]
fn compare_extra_args_ignored() {
    assert_eq!(
        call(&DateTimeModule, "compare", &[s(EARLY), s(LATE), n(1.0)]).as_number(),
        Some(-1.0)
    );
}

// ── is_between_inclusive ─────────────────────────────────────────

#[test]
fn is_between_inclusive_true() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between_inclusive",
            &[
                s("2021-06-15T10:00:00Z"),
                s("2021-06-01T00:00:00Z"),
                s("2021-06-30T00:00:00Z")
            ]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn is_between_inclusive_equal_start() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between_inclusive",
            &[
                s("2021-06-01T00:00:00Z"),
                s("2021-06-01T00:00:00Z"),
                s("2021-06-30T00:00:00Z")
            ]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn is_between_inclusive_equal_end() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between_inclusive",
            &[
                s("2021-06-30T00:00:00Z"),
                s("2021-06-01T00:00:00Z"),
                s("2021-06-30T00:00:00Z")
            ]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn is_between_inclusive_false_before() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between_inclusive",
            &[
                s("2021-05-15T00:00:00Z"),
                s("2021-06-01T00:00:00Z"),
                s("2021-06-30T00:00:00Z")
            ]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_between_inclusive_false_after() {
    assert_eq!(
        call(
            &DateTimeModule,
            "is_between_inclusive",
            &[
                s("2021-07-15T00:00:00Z"),
                s("2021-06-01T00:00:00Z"),
                s("2021-06-30T00:00:00Z")
            ]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_between_inclusive_no_args_errors() {
    let result = call_err(&DateTimeModule, "is_between_inclusive", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
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
fn vm_is_before() {
    let expr = Expr::call(
        "datetime.is_before",
        vec![Expr::value(s(EARLY)), Expr::value(s(LATE))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn vm_compare() {
    let expr = Expr::call(
        "datetime.compare",
        vec![Expr::value(s(EARLY)), Expr::value(s(LATE))],
    );
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(DateTimeModule);
    assert!(host.function("datetime.compare").is_some());
    assert!(host.function("datetime.is_before").is_some());
    assert!(host.function("datetime.is_after").is_some());
    assert!(host.function("datetime.is_equal").is_some());
    assert!(host.function("datetime.is_between").is_some());
    assert!(host.function("datetime.is_between_inclusive").is_some());
}
