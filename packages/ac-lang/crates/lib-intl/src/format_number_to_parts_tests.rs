use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{Error};
use ac_runtime_values::{Context};
use ac_runtime_modules::Module;

use super::module::IntlModule;

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

fn obj(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

/// Extract the parts array as a Vec of (type, value) pairs.
fn extract_parts(value: NonFunctionValue) -> Vec<(String, String)> {
    let arr = value.as_array().expect("result should be an array");
    arr.iter()
        .map(|item| {
            let obj = item.as_object().expect("each part should be an object");
            let part_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let part_value = obj
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (part_type, part_value)
        })
        .collect()
}

/// Concatenate all part values to reconstruct the full formatted string.
fn parts_to_string(parts: &[(String, String)]) -> String {
    parts.iter().map(|(_, v)| v.as_str()).collect()
}

// ── Basic decimal ────────────────────────────────────────────────

#[test]
fn to_parts_integer() {
    let result = call(&IntlModule::new(), "format_number_to_parts", &[n(42.0)]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"integer"),
        "parts should contain integer, got: {types:?}"
    );
    assert_eq!(parts_to_string(&parts), "42");
}

#[test]
fn to_parts_thousands() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(1234567.0)],
    );
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(types.contains(&"integer"), "should have integer parts");
    assert!(
        types.contains(&"group"),
        "should have group parts, got: {types:?}"
    );
    assert_eq!(parts_to_string(&parts), "1,234,567");
}

#[test]
fn to_parts_decimal() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(1234567.89)],
    );
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"decimal"),
        "should have decimal separator part, got: {types:?}"
    );
    assert!(
        types.contains(&"fraction"),
        "should have fraction part, got: {types:?}"
    );
    assert_eq!(parts_to_string(&parts), "1,234,567.89");
}

#[test]
fn to_parts_zero() {
    let result = call(&IntlModule::new(), "format_number_to_parts", &[n(0.0)]);
    let parts = extract_parts(result);
    assert_eq!(parts_to_string(&parts), "0");
}

#[test]
fn to_parts_negative() {
    let result = call(&IntlModule::new(), "format_number_to_parts", &[n(-1234.0)]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"minusSign"),
        "negative number should have minusSign part, got: {types:?}"
    );
    assert_eq!(parts_to_string(&parts), "-1,234");
}

#[test]
fn to_parts_sign_display_always() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(42.0), s("en"), obj(&[("signDisplay", "always")])],
    );
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"plusSign"),
        "signDisplay:always should have plusSign part, got: {types:?}"
    );
}

// ── Error cases ──────────────────────────────────────────────────

#[test]
fn to_parts_no_args_errors() {
    let result = call_err(&IntlModule::new(), "format_number_to_parts", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn to_parts_non_number_errors() {
    let result = call_err(&IntlModule::new(), "format_number_to_parts", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── Locale-specific ──────────────────────────────────────────────

#[test]
fn to_parts_pt_br_separator() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(1234567.89), s("pt-BR")],
    );
    let parts = extract_parts(result);
    assert_eq!(parts_to_string(&parts), "1.234.567,89");
}

#[test]
fn to_parts_bangla_digits() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(1000007.0), s("bn")],
    );
    let parts = extract_parts(result);
    assert_eq!(parts_to_string(&parts), "১০,০০,০০৭");
}

// ── Percent style ────────────────────────────────────────────────

#[test]
fn to_parts_percent() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(0.5), s("en"), obj(&[("style", "percent")])],
    );
    let parts = extract_parts(result);
    let s = parts_to_string(&parts);
    assert!(s.contains("50"), "percent should contain 50, got: {s}");
    assert!(s.contains("%"), "percent should contain %, got: {s}");
}

// ── Currency style ───────────────────────────────────────────────

#[test]
fn to_parts_currency_usd() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[
            n(1234.56),
            s("en"),
            obj(&[("style", "currency"), ("currency", "USD")]),
        ],
    );
    let parts = extract_parts(result);
    let s = parts_to_string(&parts);
    assert!(
        s.contains("1,234.56"),
        "currency should contain formatted number, got: {s}"
    );
}

#[test]
fn to_parts_currency_missing_code_errors() {
    let result = call_err(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(100.0), s("en"), obj(&[("style", "currency")])],
    );
    assert!(result.is_err());
}

// ── Compact notation ─────────────────────────────────────────────

#[test]
fn to_parts_compact() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[n(1000000.0), s("en"), obj(&[("notation", "compact")])],
    );
    let parts = extract_parts(result);
    let s = parts_to_string(&parts);
    assert!(!s.is_empty(), "compact should produce output: {s}");
}

// ── Unit style ───────────────────────────────────────────────────

#[test]
fn to_parts_unit_meter() {
    let result = call(
        &IntlModule::new(),
        "format_number_to_parts",
        &[
            n(5.0),
            s("en"),
            obj(&[("style", "unit"), ("unit", "meter")]),
        ],
    );
    let parts = extract_parts(result);
    let s = parts_to_string(&parts);
    assert!(s.contains("5"), "unit meter should contain 5, got: {s}");
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::{CanonicalContext, Host, Vm};
use ac_runtime_modules::ModuleHost;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_format_number_to_parts() {
    let expr = Expr::call(
        "intl.format_number_to_parts",
        vec![Expr::value(n(1234567.0))],
    );
    let result = eval_expr(&expr);
    assert!(result.is_array(), "result should be an array");
}

#[test]
fn vm_resolves_format_number_to_parts() {
    let host = ModuleHost::new().with(IntlModule::new());
    assert!(host.function("intl.format_number_to_parts").is_some());
}

// ── Consistency: parts concatenated == format_number string ──────

#[test]
fn to_parts_matches_format_number_string() {
    let module = IntlModule::new();
    let formatted = call(&module, "format_number", &[n(1234567.89), s("en")]);
    let parts = call(&module, "format_number_to_parts", &[n(1234567.89), s("en")]);
    let parts_str = parts_to_string(&extract_parts(parts));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_number output"
    );
}

#[test]
fn to_parts_matches_format_number_negative() {
    let module = IntlModule::new();
    let formatted = call(&module, "format_number", &[n(-1234.0), s("en")]);
    let parts = call(&module, "format_number_to_parts", &[n(-1234.0), s("en")]);
    let parts_str = parts_to_string(&extract_parts(parts));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_number output for negative"
    );
}

#[test]
fn to_parts_matches_format_number_pt_br() {
    let module = IntlModule::new();
    let formatted = call(&module, "format_number", &[n(1234567.89), s("pt-BR")]);
    let parts = call(
        &module,
        "format_number_to_parts",
        &[n(1234567.89), s("pt-BR")],
    );
    let parts_str = parts_to_string(&extract_parts(parts));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_number output for pt-BR"
    );
}

#[test]
fn to_parts_matches_format_number_percent() {
    let module = IntlModule::new();
    let formatted = call(
        &module,
        "format_number",
        &[n(0.5), s("en"), obj(&[("style", "percent")])],
    );
    let parts = call(
        &module,
        "format_number_to_parts",
        &[n(0.5), s("en"), obj(&[("style", "percent")])],
    );
    let parts_str = parts_to_string(&extract_parts(parts));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_number output for percent"
    );
}

#[test]
fn to_parts_matches_format_number_currency() {
    let module = IntlModule::new();
    let formatted = call(
        &module,
        "format_number",
        &[
            n(1234.56),
            s("en"),
            obj(&[("style", "currency"), ("currency", "USD")]),
        ],
    );
    let parts = call(
        &module,
        "format_number_to_parts",
        &[
            n(1234.56),
            s("en"),
            obj(&[("style", "currency"), ("currency", "USD")]),
        ],
    );
    let parts_str = parts_to_string(&extract_parts(parts));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_number output for currency"
    );
}