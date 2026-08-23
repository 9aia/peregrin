use ac_runtime::NonFunctionValue;
use ac_runtime_modules::Module;
use ac_runtime::{Context, Error};

use super::TypeModule;

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

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn null() -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Null)
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn obj(pairs: &[(&str, serde_json::Value)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

// ── is_null ───────────────────────────────────────────────────────

#[test]
fn is_null_true() {
    assert_eq!(
        call(&TypeModule, "is_null", &[null()]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_null_false() {
    assert_eq!(
        call(&TypeModule, "is_null", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

// ── is_bool ───────────────────────────────────────────────────────

#[test]
fn is_bool_true() {
    assert_eq!(
        call(&TypeModule, "is_bool", &[b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_bool_false() {
    assert_eq!(
        call(&TypeModule, "is_bool", &[n(1.0)]).as_bool(),
        Some(false)
    );
}

// ── is_number ─────────────────────────────────────────────────────

#[test]
fn is_number_true() {
    assert_eq!(
        call(&TypeModule, "is_number", &[n(42.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_number_false() {
    assert_eq!(
        call(&TypeModule, "is_number", &[s("42")]).as_bool(),
        Some(false)
    );
}

// ── is_string ─────────────────────────────────────────────────────

#[test]
fn is_string_true() {
    assert_eq!(
        call(&TypeModule, "is_string", &[s("hello")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_string_false() {
    assert_eq!(
        call(&TypeModule, "is_string", &[n(42.0)]).as_bool(),
        Some(false)
    );
}

// ── is_array ──────────────────────────────────────────────────────

#[test]
fn is_array_true() {
    assert_eq!(
        call(&TypeModule, "is_array", &[arr(&[])]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_array_false() {
    assert_eq!(
        call(&TypeModule, "is_array", &[n(42.0)]).as_bool(),
        Some(false)
    );
}

// ── is_object ─────────────────────────────────────────────────────

#[test]
fn is_object_true() {
    assert_eq!(
        call(&TypeModule, "is_object", &[obj(&[])]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_object_false() {
    assert_eq!(
        call(&TypeModule, "is_object", &[n(42.0)]).as_bool(),
        Some(false)
    );
}

// ── type_of ───────────────────────────────────────────────────────

#[test]
fn type_of_null() {
    assert_eq!(
        call(&TypeModule, "type_of", &[null()]).as_str(),
        Some("null")
    );
}

#[test]
fn type_of_bool() {
    assert_eq!(
        call(&TypeModule, "type_of", &[b(true)]).as_str(),
        Some("boolean")
    );
}

#[test]
fn type_of_number() {
    assert_eq!(
        call(&TypeModule, "type_of", &[n(42.0)]).as_str(),
        Some("number")
    );
}

#[test]
fn type_of_string() {
    assert_eq!(
        call(&TypeModule, "type_of", &[s("hi")]).as_str(),
        Some("string")
    );
}

#[test]
fn type_of_array() {
    assert_eq!(
        call(&TypeModule, "type_of", &[arr(&[])]).as_str(),
        Some("array")
    );
}

#[test]
fn type_of_object() {
    assert_eq!(
        call(&TypeModule, "type_of", &[obj(&[])]).as_str(),
        Some("object")
    );
}

// ── is_empty ──────────────────────────────────────────────────────

#[test]
fn is_empty_null() {
    assert_eq!(
        call(&TypeModule, "is_empty", &[null()]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_empty_string() {
    assert_eq!(
        call(&TypeModule, "is_empty", &[s("")]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&TypeModule, "is_empty", &[s("x")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_empty_array() {
    assert_eq!(
        call(&TypeModule, "is_empty", &[arr(&[])]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&TypeModule, "is_empty", &[arr(&[serde_json::json!(1)])]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_empty_object() {
    assert_eq!(
        call(&TypeModule, "is_empty", &[obj(&[])]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(
            &TypeModule,
            "is_empty",
            &[obj(&[("a", serde_json::json!(1))])]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_empty_number_is_false() {
    assert_eq!(
        call(&TypeModule, "is_empty", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

// ── is_truthy ─────────────────────────────────────────────────────

#[test]
fn is_truthy_bool() {
    assert_eq!(
        call(&TypeModule, "is_truthy", &[b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&TypeModule, "is_truthy", &[b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_number() {
    assert_eq!(
        call(&TypeModule, "is_truthy", &[n(1.0)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&TypeModule, "is_truthy", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_string() {
    assert_eq!(
        call(&TypeModule, "is_truthy", &[s("x")]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&TypeModule, "is_truthy", &[s("")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_null() {
    assert_eq!(
        call(&TypeModule, "is_truthy", &[null()]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_array() {
    assert_eq!(
        call(&TypeModule, "is_truthy", &[arr(&[])]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&TypeModule, "is_truthy", &[arr(&[serde_json::json!(1)])]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_truthy_object() {
    assert_eq!(
        call(&TypeModule, "is_truthy", &[obj(&[])]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(
            &TypeModule,
            "is_truthy",
            &[obj(&[("a", serde_json::json!(1))])]
        )
        .as_bool(),
        Some(true)
    );
}

// ── edge cases ────────────────────────────────────────────────────

#[test]
fn is_empty_on_empty_string() {
    let result = call(&TypeModule, "is_empty", &[s("")]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_empty_on_non_empty_string() {
    let result = call(&TypeModule, "is_empty", &[s("hello")]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_empty_on_empty_object() {
    let result = call(&TypeModule, "is_empty", &[obj(&[])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_empty_on_non_empty_object() {
    let result = call(
        &TypeModule,
        "is_empty",
        &[obj(&[("a", serde_json::json!(1))])],
    );
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_empty_on_null() {
    let result = call(
        &TypeModule,
        "is_empty",
        &[NonFunctionValue::new(serde_json::Value::Null)],
    );
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_empty_on_number() {
    let result = call(&TypeModule, "is_empty", &[n(42.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_empty_no_args_errors() {
    let result = call_err(&TypeModule, "is_empty", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_truthy_on_false() {
    let result = call(&TypeModule, "is_truthy", &[NonFunctionValue::new(false)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_truthy_on_true() {
    let result = call(&TypeModule, "is_truthy", &[NonFunctionValue::new(true)]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_truthy_on_zero() {
    let result = call(&TypeModule, "is_truthy", &[n(0.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_truthy_on_nonzero() {
    let result = call(&TypeModule, "is_truthy", &[n(42.0)]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_truthy_on_empty_string() {
    let result = call(&TypeModule, "is_truthy", &[s("")]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_truthy_on_nonempty_string() {
    let result = call(&TypeModule, "is_truthy", &[s("hello")]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_truthy_on_null() {
    let result = call(
        &TypeModule,
        "is_truthy",
        &[NonFunctionValue::new(serde_json::Value::Null)],
    );
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_truthy_no_args_errors() {
    let result = call_err(&TypeModule, "is_truthy", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn type_of_no_args_errors() {
    let result = call_err(&TypeModule, "type_of", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── VM integration tests ──────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_modules::ModuleHost;
use ac_runtime_canonical::{CanonicalContext, Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(TypeModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

fn eval_expr_result(expr: &Expr<NonFunctionValue>) -> Result<NonFunctionValue, Error> {
    let host = ModuleHost::new().with(TypeModule);
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx)
}

#[test]
fn vm_is_null_true() {
    let expr = Expr::call("type.is_null", vec![Expr::value(null())]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn vm_type_of_number() {
    let expr = Expr::call("type.type_of", vec![Expr::value(n(42.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("number"));
}

#[test]
fn vm_is_truthy_string() {
    let expr = Expr::call("type.is_truthy", vec![Expr::value(s("hello"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn vm_predicates_resolve_via_module_host() {
    let host = ModuleHost::new().with(TypeModule);
    assert!(host.function("type.is_null").is_some());
    assert!(host.function("type.is_bool").is_some());
    assert!(host.function("type.is_number").is_some());
    assert!(host.function("type.is_string").is_some());
    assert!(host.function("type.is_array").is_some());
    assert!(host.function("type.is_object").is_some());
    assert!(host.function("type.type_of").is_some());
    assert!(host.function("type.is_empty").is_some());
    assert!(host.function("type.is_truthy").is_some());
}
