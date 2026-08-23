use ac_runtime::NonFunctionValue;
use ac_runtime::{CanonicalContext, Context};
use ac_runtime_modules::Module;

use super::module::IntlModule;

fn call() -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module
        .function("supported_locales_of")
        .expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(&[], &mut ctx).expect("call succeeds")
}

#[test]
fn supported_locales_of_returns_array() {
    let result = call();
    let arr = result.as_array().expect("array result");
    assert!(!arr.is_empty());
}

#[test]
fn supported_locales_of_contains_english() {
    let result = call();
    let arr = result.as_array().expect("array result");
    let has_en = arr.iter().any(|v| v.as_str() == Some("en"));
    assert!(has_en, "expected 'en' in supported locales");
}

#[test]
fn supported_locales_of_contains_japanese() {
    let result = call();
    let arr = result.as_array().expect("array result");
    let has_ja = arr.iter().any(|v| v.as_str() == Some("ja"));
    assert!(has_ja, "expected 'ja' in supported locales");
}
