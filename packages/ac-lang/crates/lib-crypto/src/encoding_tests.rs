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

fn extract_byte_array(v: &NonFunctionValue) -> Vec<u8> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_f64().unwrap() as u8)
        .collect()
}

// ── Base64 encode ────────────────────────────────────────────────

#[test]
fn base64_encode_string() {
    assert_eq!(
        call(&CryptoModule, "base64_encode", &[s("hello")]).as_str(),
        Some("aGVsbG8=")
    );
}

#[test]
fn base64_encode_byte_array() {
    assert_eq!(
        call(&CryptoModule, "base64_encode", &[bytes(&[0, 1, 2])]).as_str(),
        Some("AAEC")
    );
}

#[test]
fn base64_encode_empty() {
    assert_eq!(
        call(&CryptoModule, "base64_encode", &[s("")]).as_str(),
        Some("")
    );
}

#[test]
fn base64_encode_no_args_errors() {
    let result = call_err(&CryptoModule, "base64_encode", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── Base64 decode ────────────────────────────────────────────────

#[test]
fn base64_decode_basic() {
    let result = call(&CryptoModule, "base64_decode", &[s("aGVsbG8=")]);
    assert_eq!(extract_byte_array(&result), vec![104, 101, 108, 108, 111]);
}

#[test]
fn base64_decode_binary() {
    let result = call(&CryptoModule, "base64_decode", &[s("AAEC")]);
    assert_eq!(extract_byte_array(&result), vec![0, 1, 2]);
}

#[test]
fn base64_decode_empty() {
    let result = call(&CryptoModule, "base64_decode", &[s("")]);
    assert_eq!(extract_byte_array(&result), Vec::<u8>::new());
}

#[test]
fn base64_decode_invalid_errors() {
    let result = call_err(&CryptoModule, "base64_decode", &[s("!!!not base64!!!")]);
    assert!(result.is_err());
}

#[test]
fn base64_roundtrip() {
    let encoded = call(&CryptoModule, "base64_encode", &[s("roundtrip test")]);
    let decoded = call(&CryptoModule, "base64_decode", &[encoded]);
    let bytes = extract_byte_array(&decoded);
    assert_eq!(String::from_utf8(bytes).unwrap(), "roundtrip test");
}

// ── Hex encode ───────────────────────────────────────────────────

#[test]
fn hex_encode_string() {
    assert_eq!(
        call(&CryptoModule, "hex_encode", &[s("hello")]).as_str(),
        Some("68656c6c6f")
    );
}

#[test]
fn hex_encode_byte_array() {
    assert_eq!(
        call(&CryptoModule, "hex_encode", &[bytes(&[0, 255, 16])]).as_str(),
        Some("00ff10")
    );
}

#[test]
fn hex_encode_empty() {
    assert_eq!(
        call(&CryptoModule, "hex_encode", &[s("")]).as_str(),
        Some("")
    );
}

// ── Hex decode ───────────────────────────────────────────────────

#[test]
fn hex_decode_basic() {
    let result = call(&CryptoModule, "hex_decode", &[s("68656c6c6f")]);
    assert_eq!(extract_byte_array(&result), vec![104, 101, 108, 108, 111]);
}

#[test]
fn hex_decode_uppercase() {
    let result = call(&CryptoModule, "hex_decode", &[s("68656C6C6F")]);
    assert_eq!(extract_byte_array(&result), vec![104, 101, 108, 108, 111]);
}

#[test]
fn hex_decode_empty() {
    let result = call(&CryptoModule, "hex_decode", &[s("")]);
    assert_eq!(extract_byte_array(&result), Vec::<u8>::new());
}

#[test]
fn hex_decode_invalid_chars_errors() {
    let result = call_err(&CryptoModule, "hex_decode", &[s("xyz")]);
    assert!(result.is_err());
}

#[test]
fn hex_decode_odd_length_errors() {
    let result = call_err(&CryptoModule, "hex_decode", &[s("abc")]);
    assert!(result.is_err());
}

#[test]
fn hex_roundtrip() {
    let encoded = call(&CryptoModule, "hex_encode", &[s("roundtrip test")]);
    let decoded = call(&CryptoModule, "hex_decode", &[encoded]);
    let bytes = extract_byte_array(&decoded);
    assert_eq!(String::from_utf8(bytes).unwrap(), "roundtrip test");
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(CryptoModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_base64_encode() {
    let expr = Expr::call("crypto.base64_encode", vec![Expr::value(s("hello"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("aGVsbG8="));
}

#[test]
fn vm_hex_encode() {
    let expr = Expr::call("crypto.hex_encode", vec![Expr::value(s("hello"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("68656c6c6f"));
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(CryptoModule);
    assert!(host.function("crypto.base64_encode").is_some());
    assert!(host.function("crypto.base64_decode").is_some());
    assert!(host.function("crypto.hex_encode").is_some());
    assert!(host.function("crypto.hex_decode").is_some());
}
