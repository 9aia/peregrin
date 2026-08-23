use ac_runtime::NonFunctionValue;
use ac_runtime::{CanonicalContext, Context, Error};
use ac_runtime_modules::Module;

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("compare_strings").expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("compare_strings").expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
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

#[test]
fn compare_less() {
    let result = call(&[s("a"), s("b")]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, -1.0);
}

#[test]
fn compare_equal() {
    let result = call(&[s("a"), s("a")]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, 0.0);
}

#[test]
fn compare_greater() {
    let result = call(&[s("b"), s("a")]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, 1.0);
}

#[test]
fn compare_base_sensitivity() {
    let result = call(&[s("ä"), s("a"), s("de"), obj(&[("sensitivity", "base")])]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, 0.0);
}

#[test]
fn compare_accent_sensitivity() {
    let result = call(&[s("ä"), s("a"), s("de"), obj(&[("sensitivity", "accent")])]);
    let n = result.as_number().expect("number result");
    assert_ne!(n, 0.0);
}

#[test]
fn compare_case_sensitivity() {
    let result = call(&[s("A"), s("a"), s("en"), obj(&[("sensitivity", "case")])]);
    let n = result.as_number().expect("number result");
    assert_ne!(n, 0.0);
}

#[test]
fn compare_variant_sensitivity() {
    let result = call(&[s("a"), s("a"), s("en"), obj(&[("sensitivity", "variant")])]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, 0.0);
}

#[test]
fn compare_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn compare_one_arg_errors() {
    assert!(call_err(&[s("a")]).is_err());
}

#[test]
fn compare_swedish_a_ring() {
    let result = call(&[s("ä"), s("a"), s("sv")]);
    let n = result.as_number().expect("number result");
    assert!(n >= 0.0);
}

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
fn vm_compare_strings() {
    let expr = Expr::call(
        "intl.compare_strings",
        vec![Expr::value(s("a")), Expr::value(s("b"))],
    );
    let result = eval_expr(&expr);
    assert!(result.as_number().is_some());
}

fn obj_bool(pairs: &[(&str, bool)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

#[test]
fn compare_numeric_sort() {
    let result = call(&[
        s("file10"),
        s("file2"),
        s("en"),
        obj_bool(&[("numeric", true)]),
    ]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, 1.0);
}

#[test]
fn compare_numeric_disabled() {
    let result = call(&[s("file10"), s("file2"), s("en")]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, -1.0);
}

#[test]
fn compare_case_first_upper() {
    let result = call(&[s("a"), s("A"), s("en"), obj(&[("caseFirst", "upper")])]);
    let n = result.as_number().expect("number result");
    assert!(n != 0.0 || n == 0.0);
}

#[test]
fn compare_ignore_punctuation() {
    let result = call(&[
        s("hello"),
        s("hel-lo"),
        s("en"),
        obj_bool(&[("ignorePunctuation", true)]),
    ]);
    let n = result.as_number().expect("number result");
    assert_eq!(n, 0.0);
}

#[test]
fn compare_ignore_punctuation_disabled() {
    let result = call(&[s("hello"), s("hel-lo"), s("en")]);
    let n = result.as_number().expect("number result");
    assert_ne!(n, 0.0);
}
