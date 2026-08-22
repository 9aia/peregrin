use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::Context;

use super::module::IntlModule;

fn call_canonical(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("get_canonical_locales").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_supported(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("supported_locales_of").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn arr(v: &[&str]) -> NonFunctionValue {
    let items: Vec<serde_json::Value> = v.iter().map(|x| serde_json::json!(x)).collect();
    NonFunctionValue::new(serde_json::Value::Array(items))
}

#[test]
fn get_canonical_single_locale() {
    let result = call_canonical(&[s("en-us")]);
    let a = result.as_array().expect("array result");
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].as_str(), Some("en-US"));
}

#[test]
fn get_canonical_multiple_locales() {
    let result = call_canonical(&[arr(&["de", "ja-jp"])]);
    let a = result.as_array().expect("array result");
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].as_str(), Some("de"));
    assert_eq!(a[1].as_str(), Some("ja-JP"));
}

#[test]
fn supported_locales_filter_single() {
    let result = call_supported(&[s("en")]);
    let a = result.as_array().expect("array result");
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].as_str(), Some("en"));
}

#[test]
fn supported_locales_filter_unsupported() {
    let result = call_supported(&[arr(&["de", "xxx"])]);
    let a = result.as_array().expect("array result");
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].as_str(), Some("de"));
}

#[test]
fn supported_locales_no_args_returns_all() {
    let result = call_supported(&[]);
    let a = result.as_array().expect("array result");
    assert!(!a.is_empty());
    let has_en = a.iter().any(|v| v.as_str() == Some("en"));
    assert!(has_en);
}
