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

fn obj(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
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

#[test]
fn display_name_variant() {
    let result = call(&[s("fonupa"), s("variant"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

// ── Style and fallback options ───────────────────────────────────

#[test]
fn display_name_region_style_short() {
    let result = call(&[s("US"), s("region"), s("en"), obj(&[("style", "short")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn display_name_region_style_narrow() {
    let result = call(&[s("US"), s("region"), s("en"), obj(&[("style", "narrow")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn display_name_script_style_short() {
    let result = call(&[s("Latn"), s("script"), s("en"), obj(&[("style", "short")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn display_name_language_style_short() {
    let result = call(&[s("en-US"), s("language"), s("en"), obj(&[("style", "short")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn display_name_language_style_narrow() {
    let result = call(&[s("en-US"), s("language"), s("en"), obj(&[("style", "narrow")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn display_name_fallback_none_returns_error_for_unknown() {
    let result = call_err(&[s("zzzzz"), s("language"), s("zh"), obj(&[("fallback", "none")])]);
    assert!(result.is_err(), "fallback:none should error for unknown code in zh locale");
}

#[test]
fn display_name_fallback_code_returns_code_for_unknown() {
    let result = call(&[s("zzz"), s("language"), s("en"), obj(&[("fallback", "code")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

// ── Currency display names ───────────────────────────────────────

#[test]
fn display_name_currency_usd() {
    let result = call(&[s("USD"), s("currency"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("US dollar") || str.contains("US Dollar"), "currency USD in en should contain 'US dollar', got: {str}");
}

#[test]
fn display_name_currency_eur() {
    let result = call(&[s("EUR"), s("currency"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("Euro"), "currency EUR in en should contain 'Euro', got: {str}");
}

#[test]
fn display_name_currency_jpy() {
    let result = call(&[s("JPY"), s("currency"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("Japanese yen") || str.contains("Yen"), "currency JPY in en should contain 'Japanese yen', got: {str}");
}

#[test]
fn display_name_currency_locale_aware() {
    let result = call(&[s("USD"), s("currency"), s("pt")]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty() && str != "USD", "currency USD in pt should be localized, got: {str}");
}

#[test]
fn display_name_currency_fallback_code() {
    let result = call(&[s("ZZZ"), s("currency"), s("en"), obj(&[("fallback", "code")])]);
    let str = result.as_str().expect("string result");
    assert_eq!(str, "ZZZ", "unknown currency with fallback:code should return code, got: {str}");
}

// ── Calendar display names ───────────────────────────────────────

#[test]
fn display_name_calendar_gregory() {
    let result = call(&[s("gregory"), s("calendar"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("Gregorian"), "calendar gregory should contain 'Gregorian', got: {str}");
}

#[test]
fn display_name_calendar_buddhist() {
    let result = call(&[s("buddhist"), s("calendar"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("Buddhist"), "calendar buddhist should contain 'Buddhist', got: {str}");
}

#[test]
fn display_name_calendar_unknown_fallback() {
    let result = call(&[s("unknowncal"), s("calendar"), s("en"), obj(&[("fallback", "code")])]);
    let str = result.as_str().expect("string result");
    assert_eq!(str, "unknowncal", "unknown calendar with fallback:code should return code, got: {str}");
}

// ── DateTimeField display names ──────────────────────────────────

#[test]
fn display_name_datetime_field_year() {
    let result = call(&[s("year"), s("dateTimeField"), s("en")]);
    let str = result.as_str().expect("string result");
    assert_eq!(str, "year", "dateTimeField year should be 'year', got: {str}");
}

#[test]
fn display_name_datetime_field_month() {
    let result = call(&[s("month"), s("dateTimeField"), s("en")]);
    let str = result.as_str().expect("string result");
    assert_eq!(str, "month", "dateTimeField month should be 'month', got: {str}");
}

#[test]
fn display_name_datetime_field_time_zone_name() {
    let result = call(&[s("timeZoneName"), s("dateTimeField"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("time zone"), "dateTimeField timeZoneName should contain 'time zone', got: {str}");
}

#[test]
fn display_name_datetime_field_unknown_fallback() {
    let result = call(&[s("unknownField"), s("dateTimeField"), s("en"), obj(&[("fallback", "code")])]);
    let str = result.as_str().expect("string result");
    assert_eq!(str, "unknownField", "unknown dateTimeField with fallback:code should return code, got: {str}");
}

// ── Calendar/dateTimeField English-only fallback documentation ──

#[test]
fn display_name_calendar_ignores_locale() {
    let en = call(&[s("gregory"), s("calendar"), s("en")]).as_str().expect("string").to_string();
    let ja = call(&[s("gregory"), s("calendar"), s("ja")]).as_str().expect("string").to_string();
    let pt = call(&[s("gregory"), s("calendar"), s("pt")]).as_str().expect("string").to_string();
    assert_eq!(en, ja, "calendar display names are English-only fallbacks, locale should be ignored");
    assert_eq!(en, pt, "calendar display names are English-only fallbacks, locale should be ignored");
}

#[test]
fn display_name_datetime_field_ignores_locale() {
    let en = call(&[s("year"), s("dateTimeField"), s("en")]).as_str().expect("string").to_string();
    let ja = call(&[s("year"), s("dateTimeField"), s("ja")]).as_str().expect("string").to_string();
    let de = call(&[s("year"), s("dateTimeField"), s("de")]).as_str().expect("string").to_string();
    assert_eq!(en, ja, "dateTimeField display names are English-only fallbacks, locale should be ignored");
    assert_eq!(en, de, "dateTimeField display names are English-only fallbacks, locale should be ignored");
}
