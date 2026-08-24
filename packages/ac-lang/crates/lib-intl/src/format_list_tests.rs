use ac_runtime_modules::Module;
use ac_runtime_values::Context;
use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{CanonicalContext, Error};

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

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn arr(items: &[&str]) -> NonFunctionValue {
    let vec: Vec<serde_json::Value> = items.iter().map(|i| serde_json::json!(i)).collect();
    NonFunctionValue::new(serde_json::Value::Array(vec))
}

fn obj(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

// ── English (default) ────────────────────────────────────────────

#[test]
fn format_list_empty() {
    let result = call(&IntlModule::new(), "format_list", &[arr(&[])]);
    assert_eq!(result.as_str(), Some(""));
}

#[test]
fn format_list_single() {
    let result = call(&IntlModule::new(), "format_list", &[arr(&["a"])]);
    assert_eq!(result.as_str(), Some("a"));
}

#[test]
fn format_list_pair() {
    let result = call(&IntlModule::new(), "format_list", &[arr(&["a", "b"])]);
    assert_eq!(result.as_str(), Some("a and b"));
}

#[test]
fn format_list_three() {
    let result = call(&IntlModule::new(), "format_list", &[arr(&["a", "b", "c"])]);
    assert_eq!(result.as_str(), Some("a, b, and c"));
}

#[test]
fn format_list_no_args_errors() {
    let result = call_err(&IntlModule::new(), "format_list", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Spanish ──────────────────────────────────────────────────────

#[test]
fn format_list_es_pair() {
    let result = call(
        &IntlModule::new(),
        "format_list",
        &[arr(&["España", "Suiza"]), s("es")],
    );
    assert_eq!(result.as_str(), Some("España y Suiza"));
}

// ── Disjunction ──────────────────────────────────────────────────

#[test]
fn format_list_disjunction() {
    let result = call(
        &IntlModule::new(),
        "format_list",
        &[
            arr(&["a", "b", "c"]),
            s("en"),
            obj(&[("type", "disjunction")]),
        ],
    );
    assert_eq!(result.as_str(), Some("a, b, or c"));
}

// ── Short style ──────────────────────────────────────────────────

#[test]
fn format_list_short_style() {
    let result = call(
        &IntlModule::new(),
        "format_list",
        &[arr(&["a", "b"]), s("en"), obj(&[("style", "short")])],
    );
    let formatted = result.as_str().expect("string result");
    assert!(formatted.contains("a") && formatted.contains("b"));
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::Vm;
use ac_runtime_modules::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format_list() {
    let expr = Expr::call("intl.format_list", vec![Expr::value(arr(&["a", "b", "c"]))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}

#[test]
fn format_list_unit() {
    let result = call(
        &IntlModule::new(),
        "format_list",
        &[arr(&["5", "7"]), s("en"), obj(&[("type", "unit")])],
    );
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
    assert!(str.contains("5") && str.contains("7"));
}

#[test]
fn format_list_unit_short() {
    let result = call(
        &IntlModule::new(),
        "format_list",
        &[
            arr(&["5", "7"]),
            s("en"),
            obj(&[("type", "unit"), ("style", "short")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}
