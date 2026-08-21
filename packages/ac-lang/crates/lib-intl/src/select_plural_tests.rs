use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

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

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn obj(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

// ── English cardinal ─────────────────────────────────────────────

#[test]
fn select_plural_en_one() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(1.0)]).as_str(),
        Some("one")
    );
}

#[test]
fn select_plural_en_other() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(5.0)]).as_str(),
        Some("other")
    );
}

#[test]
fn select_plural_en_zero() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(0.0)]).as_str(),
        Some("other")
    );
}

// ── English ordinal ──────────────────────────────────────────────

#[test]
fn select_plural_en_ordinal_one() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(1.0), s("en"), obj(&[("type", "ordinal")])]).as_str(),
        Some("one")
    );
}

#[test]
fn select_plural_en_ordinal_two() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(2.0), s("en"), obj(&[("type", "ordinal")])]).as_str(),
        Some("two")
    );
}

#[test]
fn select_plural_en_ordinal_few() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(3.0), s("en"), obj(&[("type", "ordinal")])]).as_str(),
        Some("few")
    );
}

#[test]
fn select_plural_en_ordinal_other() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(11.0), s("en"), obj(&[("type", "ordinal")])]).as_str(),
        Some("other")
    );
}

// ── Arabic cardinal ──────────────────────────────────────────────

#[test]
fn select_plural_ar_zero() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(0.0), s("ar")]).as_str(),
        Some("zero")
    );
}

#[test]
fn select_plural_ar_one() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(1.0), s("ar")]).as_str(),
        Some("one")
    );
}

#[test]
fn select_plural_ar_two() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(2.0), s("ar")]).as_str(),
        Some("two")
    );
}

#[test]
fn select_plural_ar_few() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(3.0), s("ar")]).as_str(),
        Some("few")
    );
}

#[test]
fn select_plural_ar_many() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(11.0), s("ar")]).as_str(),
        Some("many")
    );
}

#[test]
fn select_plural_ar_other() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(100.0), s("ar")]).as_str(),
        Some("other")
    );
}

// ── Japanese (other only) ────────────────────────────────────────

#[test]
fn select_plural_ja_one() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(1.0), s("ja")]).as_str(),
        Some("other")
    );
}

// ── Russian ──────────────────────────────────────────────────────

#[test]
fn select_plural_ru_few() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(2.0), s("ru")]).as_str(),
        Some("few")
    );
}

#[test]
fn select_plural_ru_many() {
    assert_eq!(
        call(&IntlModule, "select_plural", &[n(5.0), s("ru")]).as_str(),
        Some("many")
    );
}

// ── Error cases ──────────────────────────────────────────────────

#[test]
fn select_plural_no_args_errors() {
    let result = call_err(&IntlModule, "select_plural", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn select_plural_non_number_errors() {
    let result = call_err(&IntlModule, "select_plural", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::Vm;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_select_plural() {
    let expr = Expr::call("intl.select_plural", vec![Expr::value(n(1.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("one"));
}
