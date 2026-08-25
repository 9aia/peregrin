use handl_vm_common::Module;
use handl_vm_values_json::Context;
use handl_vm_values_json::NonFunctionValue;
use handl_vm_values_json::{CanonicalContext, Error};

use super::module::IntlModule;

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
        call(&IntlModule::new(), "select_plural", &[n(1.0)]).as_str(),
        Some("one")
    );
}

#[test]
fn select_plural_en_other() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(5.0)]).as_str(),
        Some("other")
    );
}

#[test]
fn select_plural_en_zero() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(0.0)]).as_str(),
        Some("other")
    );
}

// ── English ordinal ──────────────────────────────────────────────

#[test]
fn select_plural_en_ordinal_one() {
    assert_eq!(
        call(
            &IntlModule::new(),
            "select_plural",
            &[n(1.0), s("en"), obj(&[("type", "ordinal")])]
        )
        .as_str(),
        Some("one")
    );
}

#[test]
fn select_plural_en_ordinal_two() {
    assert_eq!(
        call(
            &IntlModule::new(),
            "select_plural",
            &[n(2.0), s("en"), obj(&[("type", "ordinal")])]
        )
        .as_str(),
        Some("two")
    );
}

#[test]
fn select_plural_en_ordinal_few() {
    assert_eq!(
        call(
            &IntlModule::new(),
            "select_plural",
            &[n(3.0), s("en"), obj(&[("type", "ordinal")])]
        )
        .as_str(),
        Some("few")
    );
}

#[test]
fn select_plural_en_ordinal_other() {
    assert_eq!(
        call(
            &IntlModule::new(),
            "select_plural",
            &[n(11.0), s("en"), obj(&[("type", "ordinal")])]
        )
        .as_str(),
        Some("other")
    );
}

// ── Arabic cardinal ──────────────────────────────────────────────

#[test]
fn select_plural_ar_zero() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(0.0), s("ar")]).as_str(),
        Some("zero")
    );
}

#[test]
fn select_plural_ar_one() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(1.0), s("ar")]).as_str(),
        Some("one")
    );
}

#[test]
fn select_plural_ar_two() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(2.0), s("ar")]).as_str(),
        Some("two")
    );
}

#[test]
fn select_plural_ar_few() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(3.0), s("ar")]).as_str(),
        Some("few")
    );
}

#[test]
fn select_plural_ar_many() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(11.0), s("ar")]).as_str(),
        Some("many")
    );
}

#[test]
fn select_plural_ar_other() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(100.0), s("ar")]).as_str(),
        Some("other")
    );
}

// ── Japanese (other only) ────────────────────────────────────────

#[test]
fn select_plural_ja_one() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(1.0), s("ja")]).as_str(),
        Some("other")
    );
}

// ── Russian ──────────────────────────────────────────────────────

#[test]
fn select_plural_ru_few() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(2.0), s("ru")]).as_str(),
        Some("few")
    );
}

#[test]
fn select_plural_ru_many() {
    assert_eq!(
        call(&IntlModule::new(), "select_plural", &[n(5.0), s("ru")]).as_str(),
        Some("many")
    );
}

// ── Error cases ──────────────────────────────────────────────────

#[test]
fn select_plural_no_args_errors() {
    let result = call_err(&IntlModule::new(), "select_plural", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn select_plural_non_number_errors() {
    let result = call_err(&IntlModule::new(), "select_plural", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── Digit options ────────────────────────────────────────────────

#[test]
fn select_plural_minimum_integer_digits() {
    let result = call(
        &IntlModule::new(),
        "select_plural",
        &[n(5.0), s("en"), obj(&[("minimumIntegerDigits", "3")])],
    );
    assert_eq!(result.as_str(), Some("other"), "005 should be 'other'");
}

#[test]
fn select_plural_min_fraction_digits_changes_category() {
    let result = call(
        &IntlModule::new(),
        "select_plural",
        &[n(1.0), s("en"), obj(&[("minimumFractionDigits", "2")])],
    );
    assert_eq!(
        result.as_str(),
        Some("other"),
        "1.00 should be 'other' not 'one'"
    );
}

#[test]
fn select_plural_trailing_zero_not_one() {
    let result = call(
        &IntlModule::new(),
        "select_plural",
        &[n(1.0), s("en"), obj(&[("minimumFractionDigits", "1")])],
    );
    assert_eq!(
        result.as_str(),
        Some("other"),
        "1.0 should be 'other' not 'one'"
    );
}

#[test]
fn select_plural_max_fraction_digits() {
    let result = call(
        &IntlModule::new(),
        "select_plural",
        &[n(1.5), s("en"), obj(&[("maximumFractionDigits", "0")])],
    );
    assert_eq!(
        result.as_str(),
        Some("other"),
        "1.5 rounded to 2 should be 'other'"
    );
}

#[test]
fn select_plural_min_significant_digits() {
    let result = call(
        &IntlModule::new(),
        "select_plural",
        &[n(1.0), s("en"), obj(&[("minimumSignificantDigits", "3")])],
    );
    assert_eq!(
        result.as_str(),
        Some("one"),
        "1.0 with 3 sig digits pads leading zeros, still 'one'"
    );
}

#[test]
fn select_plural_ar_with_fraction_digits() {
    let result = call(
        &IntlModule::new(),
        "select_plural",
        &[n(1.5), s("ar"), obj(&[("minimumFractionDigits", "2")])],
    );
    assert_eq!(
        result.as_str(),
        Some("other"),
        "1.50 in Arabic is 'other' (n=1.50, not n=1)"
    );
}

// ── VM integration ───────────────────────────────────────────────

use handl_ast_canonical::Expr;
use handl_vm_backend_rust_interpreter::Vm;
use handl_vm_common::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_select_plural() {
    let expr = Expr::call("intl.select_plural", vec![Expr::value(n(1.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("one"));
}
