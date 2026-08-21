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

// ── English ──────────────────────────────────────────────────────

#[test]
fn pluralize_one() {
    let forms = obj(&[("one", "item"), ("other", "items")]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(1.0), forms]).as_str(),
        Some("item")
    );
}

#[test]
fn pluralize_other() {
    let forms = obj(&[("one", "item"), ("other", "items")]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(5.0), forms]).as_str(),
        Some("items")
    );
}

#[test]
fn pluralize_zero_with_zero_form() {
    let forms = obj(&[("zero", "no items"), ("one", "item"), ("other", "items")]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(0.0), forms, s("ar")]).as_str(),
        Some("no items")
    );
}

#[test]
fn pluralize_zero_without_zero_form_falls_back() {
    let forms = obj(&[("one", "item"), ("other", "items")]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(0.0), forms]).as_str(),
        Some("items")
    );
}

#[test]
fn pluralize_missing_other_errors() {
    let forms = obj(&[("one", "item")]);
    let result = call_err(&IntlModule, "pluralize", &[n(5.0), forms]);
    assert!(result.is_err());
}

#[test]
fn pluralize_no_args_errors() {
    let result = call_err(&IntlModule, "pluralize", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Arabic ───────────────────────────────────────────────────────

#[test]
fn pluralize_arabic_zero() {
    let forms = obj(&[
        ("zero", "صفر"),
        ("one", "واحد"),
        ("two", "اثنان"),
        ("few", "قليل"),
        ("many", "كثير"),
        ("other", "أخرى"),
    ]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(0.0), forms, s("ar")]).as_str(),
        Some("صفر")
    );
}

#[test]
fn pluralize_arabic_one() {
    let forms = obj(&[
        ("zero", "صفر"),
        ("one", "واحد"),
        ("two", "اثنان"),
        ("few", "قليل"),
        ("many", "كثير"),
        ("other", "أخرى"),
    ]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(1.0), forms, s("ar")]).as_str(),
        Some("واحد")
    );
}

#[test]
fn pluralize_arabic_two() {
    let forms = obj(&[
        ("zero", "صفر"),
        ("one", "واحد"),
        ("two", "اثنان"),
        ("few", "قليل"),
        ("many", "كثير"),
        ("other", "أخرى"),
    ]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(2.0), forms, s("ar")]).as_str(),
        Some("اثنان")
    );
}

#[test]
fn pluralize_arabic_few() {
    let forms = obj(&[
        ("zero", "صفر"),
        ("one", "واحد"),
        ("two", "اثنان"),
        ("few", "قليل"),
        ("many", "كثير"),
        ("other", "أخرى"),
    ]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(3.0), forms, s("ar")]).as_str(),
        Some("قليل")
    );
}

#[test]
fn pluralize_arabic_many() {
    let forms = obj(&[
        ("zero", "صفر"),
        ("one", "واحد"),
        ("two", "اثنان"),
        ("few", "قليل"),
        ("many", "كثير"),
        ("other", "أخرى"),
    ]);
    assert_eq!(
        call(&IntlModule, "pluralize", &[n(11.0), forms, s("ar")]).as_str(),
        Some("كثير")
    );
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
fn vm_pluralize() {
    let forms = obj(&[("one", "item"), ("other", "items")]);
    let expr = Expr::call("intl.pluralize", vec![Expr::value(n(1.0)), Expr::value(forms)]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("item"));
}
