use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::CryptoModule;

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

fn bytes(values: &[u8]) -> NonFunctionValue {
    let arr: Vec<serde_json::Value> = values.iter().map(|b| serde_json::json!(b)).collect();
    NonFunctionValue::new(serde_json::Value::Array(arr))
}

// ── SHA-256 ──────────────────────────────────────────────────────

#[test]
fn sha256_string() {
    assert_eq!(
        call(&CryptoModule, "sha256", &[s("hello")]).as_str(),
        Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
    );
}

#[test]
fn sha256_byte_array() {
    assert_eq!(
        call(&CryptoModule, "sha256", &[bytes(&[104, 101, 108, 108, 111])]).as_str(),
        Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
    );
}

#[test]
fn sha256_empty() {
    assert_eq!(
        call(&CryptoModule, "sha256", &[s("")]).as_str(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
}

#[test]
fn sha256_unicode() {
    assert_eq!(
        call(&CryptoModule, "sha256", &[s("héllo")]).as_str(),
        Some("3c48591d8d098a4538f5e013dfcf406e948eac4d3277b10bf614e295d6068179")
    );
}

#[test]
fn sha256_no_args_errors() {
    let result = call_err(&CryptoModule, "sha256", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sha256_non_string_non_array_errors() {
    let result = call_err(&CryptoModule, "sha256", &[NonFunctionValue::new(42)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sha256_byte_out_of_range_errors() {
    let arr = NonFunctionValue::new(serde_json::json!([256]));
    let result = call_err(&CryptoModule, "sha256", &[arr]);
    assert!(result.is_err());
}

// ── MD5 ──────────────────────────────────────────────────────────

#[test]
fn md5_string() {
    assert_eq!(
        call(&CryptoModule, "md5", &[s("hello")]).as_str(),
        Some("5d41402abc4b2a76b9719d911017c592")
    );
}

#[test]
fn md5_byte_array() {
    assert_eq!(
        call(&CryptoModule, "md5", &[bytes(&[104, 101, 108, 108, 111])]).as_str(),
        Some("5d41402abc4b2a76b9719d911017c592")
    );
}

#[test]
fn md5_empty() {
    assert_eq!(
        call(&CryptoModule, "md5", &[s("")]).as_str(),
        Some("d41d8cd98f00b204e9800998ecf8427e")
    );
}

#[test]
fn md5_no_args_errors() {
    let result = call_err(&CryptoModule, "md5", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(CryptoModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_sha256() {
    let expr = Expr::call("crypto.sha256", vec![Expr::value(s("hello"))]);
    let result = eval_expr(&expr);
    assert_eq!(
        result.as_str(),
        Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
    );
}

#[test]
fn vm_md5() {
    let expr = Expr::call("crypto.md5", vec![Expr::value(s("hello"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("5d41402abc4b2a76b9719d911017c592"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(CryptoModule);
    assert!(host.function("crypto.sha256").is_some());
    assert!(host.function("crypto.md5").is_some());
}
