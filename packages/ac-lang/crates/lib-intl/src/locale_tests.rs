use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("locale").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("locale").expect("function exists");
    let mut ctx = Context::new();
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

fn get_str(result: &NonFunctionValue, key: &str) -> Option<String> {
    result.as_object()?.get(key)?.as_str().map(|s| s.to_string())
}

fn is_null(result: &NonFunctionValue, key: &str) -> bool {
    result.as_object()
        .and_then(|o| o.get(key))
        .map(|v| v.is_null())
        .unwrap_or(true)
}

// ── Basic parsing ────────────────────────────────────────────────

#[test]
fn locale_simple_language() {
    let result = call(&[s("en")]);
    assert_eq!(get_str(&result, "baseName"), Some("en".to_string()));
    assert_eq!(get_str(&result, "language"), Some("en".to_string()));
    assert!(is_null(&result, "script"));
    assert!(is_null(&result, "region"));
}

#[test]
fn locale_language_region() {
    let result = call(&[s("en-US")]);
    assert_eq!(get_str(&result, "baseName"), Some("en-US".to_string()));
    assert_eq!(get_str(&result, "language"), Some("en".to_string()));
    assert_eq!(get_str(&result, "region"), Some("US".to_string()));
    assert!(is_null(&result, "script"));
}

#[test]
fn locale_language_script_region() {
    let result = call(&[s("zh-Hant-TW")]);
    assert_eq!(get_str(&result, "baseName"), Some("zh-Hant-TW".to_string()));
    assert_eq!(get_str(&result, "language"), Some("zh".to_string()));
    assert_eq!(get_str(&result, "script"), Some("Hant".to_string()));
    assert_eq!(get_str(&result, "region"), Some("TW".to_string()));
}

// ── Options override ─────────────────────────────────────────────

#[test]
fn locale_override_region() {
    let result = call(&[s("en"), obj(&[("region", "GB")])]);
    assert_eq!(get_str(&result, "baseName"), Some("en-GB".to_string()));
    assert_eq!(get_str(&result, "region"), Some("GB".to_string()));
}

#[test]
fn locale_override_script() {
    let result = call(&[s("zh"), obj(&[("script", "Hant")])]);
    assert_eq!(get_str(&result, "baseName"), Some("zh-Hant".to_string()));
    assert_eq!(get_str(&result, "script"), Some("Hant".to_string()));
}

#[test]
fn locale_override_language() {
    let result = call(&[s("en-US"), obj(&[("language", "fr")])]);
    assert_eq!(get_str(&result, "baseName"), Some("fr-US".to_string()));
    assert_eq!(get_str(&result, "language"), Some("fr".to_string()));
}

// ── Unicode extensions ───────────────────────────────────────────

#[test]
fn locale_calendar_extension() {
    let result = call(&[s("en-US"), obj(&[("calendar", "buddhist")])]);
    let base = get_str(&result, "baseName").expect("baseName");
    assert!(base.contains("buddhist"), "baseName should contain buddhist: {base}");
    assert_eq!(get_str(&result, "calendar"), Some("buddhist".to_string()));
}

#[test]
fn locale_numbering_system_extension() {
    let result = call(&[s("en-US"), obj(&[("numberingSystem", "arab")])]);
    let base = get_str(&result, "baseName").expect("baseName");
    assert!(base.contains("arab"), "baseName should contain arab: {base}");
    assert_eq!(get_str(&result, "numberingSystem"), Some("arab".to_string()));
}

#[test]
fn locale_hour_cycle_extension() {
    let result = call(&[s("en-US"), obj(&[("hourCycle", "h23")])]);
    let base = get_str(&result, "baseName").expect("baseName");
    assert!(base.contains("h23"), "baseName should contain h23: {base}");
    assert_eq!(get_str(&result, "hourCycle"), Some("h23".to_string()));
}

// ── Error cases ──────────────────────────────────────────────────

#[test]
fn locale_invalid_tag_errors() {
    let result = call_err(&[s("!")]);
    assert!(result.is_err(), "invalid locale tag should error");
}

#[test]
fn locale_missing_arg_errors() {
    let result = call_err(&[]);
    assert!(result.is_err(), "missing argument should error");
}
