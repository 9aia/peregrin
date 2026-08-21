use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("display_name").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("display_name").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

#[test]
fn display_name_language_english() {
    let result = call(&[s("en"), s("language")]);
    let s = result.as_str().expect("string result");
    assert_eq!(s, "English");
}

#[test]
fn display_name_language_german() {
    let result = call(&[s("de"), s("language")]);
    let s = result.as_str().expect("string result");
    assert_eq!(s, "German");
}

#[test]
fn display_name_language_in_portuguese() {
    let result = call(&[s("de"), s("language"), s("pt")]);
    assert_eq!(result.as_str(), Some("alemão"));
}

#[test]
fn display_name_region_us() {
    let result = call(&[s("US"), s("region")]);
    assert_eq!(result.as_str(), Some("United States"));
}

#[test]
fn display_name_region_de() {
    let result = call(&[s("DE"), s("region")]);
    assert_eq!(result.as_str(), Some("Germany"));
}

#[test]
fn display_name_script_latn() {
    let result = call(&[s("Latn"), s("script")]);
    assert_eq!(result.as_str(), Some("Latin"));
}

#[test]
fn display_name_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn display_name_invalid_type_errors() {
    assert!(call_err(&[s("en"), s("foo")]).is_err());
}

#[test]
fn display_name_language_ja() {
    let result = call(&[s("ja"), s("language")]);
    let s = result.as_str().expect("string result");
    assert_eq!(s, "Japanese");
}

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::Vm;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_display_name() {
    let expr = Expr::call("intl.display_name", vec![Expr::value(s("en")), Expr::value(s("language"))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}
