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

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn arr(items: &[&str]) -> NonFunctionValue {
    let vec: Vec<serde_json::Value> = items.iter().map(|i| serde_json::json!(i)).collect();
    NonFunctionValue::new(serde_json::Value::Array(vec))
}

// ── English ──────────────────────────────────────────────────────

#[test]
fn format_list_empty() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&[])]).as_str(),
        Some("")
    );
}

#[test]
fn format_list_single() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a"])]).as_str(),
        Some("a")
    );
}

#[test]
fn format_list_pair() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b"])]).as_str(),
        Some("a and b")
    );
}

#[test]
fn format_list_three() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b", "c"])]).as_str(),
        Some("a, b, and c")
    );
}

#[test]
fn format_list_four() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b", "c", "d"])]).as_str(),
        Some("a, b, c, and d")
    );
}

#[test]
fn format_list_no_args_errors() {
    let result = call_err(&IntlModule, "format_list", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn format_list_pt_pair() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b"]), s("pt")]).as_str(),
        Some("a e b")
    );
}

#[test]
fn format_list_pt_three() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b", "c"]), s("pt")]).as_str(),
        Some("a, b e c")
    );
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn format_list_de_pair() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b"]), s("de")]).as_str(),
        Some("a und b")
    );
}

#[test]
fn format_list_de_three() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b", "c"]), s("de")]).as_str(),
        Some("a, b und c")
    );
}

// ── Japanese ─────────────────────────────────────────────────────

#[test]
fn format_list_ja_pair() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b"]), s("ja")]).as_str(),
        Some("aとb")
    );
}

#[test]
fn format_list_ja_three() {
    assert_eq!(
        call(&IntlModule, "format_list", &[arr(&["a", "b", "c"]), s("ja")]).as_str(),
        Some("a、b、c")
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
fn vm_format_list() {
    let expr = Expr::call("intl.format_list", vec![Expr::value(arr(&["a", "b", "c"]))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("a, b, and c"));
}
