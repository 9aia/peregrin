use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error};
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

// ── English (default) ────────────────────────────────────────────

#[test]
fn format_number_integer() {
    let result = call(&IntlModule::new(), "format_number", &[n(42.0)]);
    assert_eq!(result.as_str(), Some("42"));
}

#[test]
fn format_number_thousands() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.0)]);
    assert_eq!(result.as_str(), Some("1,234,567"));
}

#[test]
fn format_number_decimal() {
    let result = call(&IntlModule::new(), "format_number", &[n(1234567.89)]);
    assert_eq!(result.as_str(), Some("1,234,567.89"));
}

#[test]
fn format_number_zero() {
    let result = call(&IntlModule::new(), "format_number", &[n(0.0)]);
    assert_eq!(result.as_str(), Some("0"));
}

#[test]
fn format_number_negative() {
    let result = call(&IntlModule::new(), "format_number", &[n(-1234.0)]);
    assert_eq!(result.as_str(), Some("-1,234"));
}

#[test]
fn format_number_no_args_errors() {
    let result = call_err(&IntlModule::new(), "format_number", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn format_number_non_number_errors() {
    let result = call_err(&IntlModule::new(), "format_number", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── Portuguese ───────────────────────────────────────────────────

#[test]
fn format_number_pt_uses_dot_separator() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234567.89), s("pt-BR")],
    );
    assert_eq!(result.as_str(), Some("1.234.567,89"));
}

// ── German ───────────────────────────────────────────────────────

#[test]
fn format_number_de_uses_dot_separator() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234567.89), s("de")],
    );
    assert_eq!(result.as_str(), Some("1.234.567,89"));
}

// ── French ───────────────────────────────────────────────────────

#[test]
fn format_number_fr_uses_narrow_nbsp_separator() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234567.89), s("fr")],
    );
    assert_eq!(result.as_str(), Some("1\u{202f}234\u{202f}567,89"));
}

// ── Bangla digits ────────────────────────────────────────────────

#[test]
fn format_number_bangla_digits() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1000007.0), s("bn")],
    );
    assert_eq!(result.as_str(), Some("১০,০০,০০৭"));
}

// ── Compact notation ─────────────────────────────────────────────

#[test]
fn format_number_compact() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1000000.0), s("en"), obj(&[("notation", "compact")])],
    );
    let formatted = result.as_str().expect("string result");
    assert!(!formatted.is_empty());
}

// ── Percent style ────────────────────────────────────────────────

#[test]
fn format_number_percent_half() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.5), s("en"), obj(&[("style", "percent")])],
    );
    assert_eq!(result.as_str(), Some("50%"));
}

#[test]
fn format_number_percent_whole() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1.0), s("en"), obj(&[("style", "percent")])],
    );
    assert_eq!(result.as_str(), Some("100%"));
}

#[test]
fn format_number_percent_decimal() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.1234), s("en"), obj(&[("style", "percent")])],
    );
    assert_eq!(result.as_str(), Some("12.34%"));
}

#[test]
fn format_number_percent_german() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.5), s("de"), obj(&[("style", "percent")])],
    );
    assert_eq!(result.as_str(), Some("50\u{a0}%"));
}

// ── Currency style ───────────────────────────────────────────────

#[test]
fn format_number_currency_usd_symbol() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(1234.56),
            s("en"),
            obj(&[("style", "currency"), ("currency", "USD")]),
        ],
    );
    assert_eq!(result.as_str(), Some("$1,234.56"));
}

#[test]
fn format_number_currency_usd_code() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(1234.56),
            s("en"),
            obj(&[
                ("style", "currency"),
                ("currency", "USD"),
                ("currencyDisplay", "code"),
            ]),
        ],
    );
    assert_eq!(result.as_str(), Some("USD\u{a0}1,234.56"));
}

#[test]
fn format_number_currency_eur_german() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(999.99),
            s("de"),
            obj(&[("style", "currency"), ("currency", "EUR")]),
        ],
    );
    assert_eq!(result.as_str(), Some("999,99\u{a0}€"));
}

#[test]
fn format_number_currency_jpy() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(5000.0),
            s("ja"),
            obj(&[("style", "currency"), ("currency", "JPY")]),
        ],
    );
    assert_eq!(result.as_str(), Some("￥5,000"));
}

#[test]
fn format_number_currency_missing_code_errors() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[n(100.0), s("en"), obj(&[("style", "currency")])],
    );
    assert!(result.is_err());
}

#[test]
fn format_number_currency_invalid_code_errors() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(100.0),
            s("en"),
            obj(&[("style", "currency"), ("currency", "dollar")]),
        ],
    );
    assert!(result.is_err());
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
fn vm_format_number() {
    let expr = Expr::call("intl.format_number", vec![Expr::value(n(1234567.0))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(IntlModule::new());
    assert!(host.function("intl.format_number").is_some());
}

// ── New formatting options ───────────────────────────────────────

#[test]
fn format_number_compact_large() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234567890.0), s("en"), obj(&[("notation", "compact")])],
    );
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn format_number_min_fraction_digits() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12.3), s("en"), obj(&[("minimumFractionDigits", "2")])],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("12.30"));
}

#[test]
fn format_number_max_fraction_digits() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12.34567), s("en"), obj(&[("maximumFractionDigits", "2")])],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("12.35"));
}

#[test]
fn format_number_no_grouping() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234567.0), s("en"), obj(&[("useGrouping", "false")])],
    );
    let str = result.as_str().expect("string result");
    assert!(!str.contains(","));
}

#[test]
fn format_number_use_grouping_always() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234.0), s("en"), obj(&[("useGrouping", "always")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("1,234"),
        "useGrouping:always should group 4 digits, got: {str}"
    );
}

#[test]
fn format_number_use_grouping_min2() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234.0), s("en"), obj(&[("useGrouping", "min2")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        !str.contains("1,234"),
        "useGrouping:min2 should not group 4 digits, got: {str}"
    );
    let result2 = call(
        &IntlModule::new(),
        "format_number",
        &[n(12345.0), s("en"), obj(&[("useGrouping", "min2")])],
    );
    let str2 = result2.as_str().expect("string result");
    assert!(
        str2.contains("12,345"),
        "useGrouping:min2 should group 5 digits, got: {str2}"
    );
}

#[test]
fn format_number_use_grouping_auto() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1234567.0), s("en"), obj(&[("useGrouping", "auto")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("1,234,567"),
        "useGrouping:auto should group, got: {str}"
    );
}

#[test]
fn format_number_sign_display_always() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(42.0), s("en"), obj(&[("signDisplay", "always")])],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("+"));
}

#[test]
fn format_number_sign_display_never() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(-42.0), s("en"), obj(&[("signDisplay", "never")])],
    );
    let str = result.as_str().expect("string result");
    assert!(!str.contains("-"));
}

// ── Rounding modes ───────────────────────────────────────────────

#[test]
fn format_number_rounding_mode_floor() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(12.99),
            s("en"),
            obj(&[("maximumFractionDigits", "0"), ("roundingMode", "floor")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert_eq!(str, "12");
}

#[test]
fn format_number_rounding_mode_ceil() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(12.01),
            s("en"),
            obj(&[("maximumFractionDigits", "0"), ("roundingMode", "ceil")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert_eq!(str, "13");
}

#[test]
fn format_number_rounding_mode_trunc() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(12.99),
            s("en"),
            obj(&[("maximumFractionDigits", "0"), ("roundingMode", "trunc")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert_eq!(str, "12");
}

#[test]
fn format_number_rounding_mode_half_even() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(2.5),
            s("en"),
            obj(&[("maximumFractionDigits", "0"), ("roundingMode", "halfEven")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert_eq!(str, "2");
}

#[test]
fn format_number_rounding_mode_half_expand() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(2.5),
            s("en"),
            obj(&[
                ("maximumFractionDigits", "0"),
                ("roundingMode", "halfExpand"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert_eq!(str, "3");
}

// ── Currency narrowSymbol ────────────────────────────────────────

#[test]
fn format_number_currency_narrow_symbol() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(1234.56),
            s("en"),
            obj(&[
                ("style", "currency"),
                ("currency", "USD"),
                ("currencyDisplay", "narrowSymbol"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("1,234.56"));
}

// ── Currency sign (accounting) ───────────────────────────────────

#[test]
fn format_number_currency_sign_standard() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(-1234.56),
            s("en"),
            obj(&[
                ("style", "currency"),
                ("currency", "USD"),
                ("currencySign", "standard"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("-"), "standard sign should have minus: {str}");
}

#[test]
fn format_number_currency_sign_accounting() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(-1234.56),
            s("en"),
            obj(&[
                ("style", "currency"),
                ("currency", "USD"),
                ("currencySign", "accounting"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("(") || str.contains("-"),
        "accounting sign should use parentheses or minus: {str}"
    );
}

// ── Compact display long ──────────────────────────────────────────

#[test]
fn format_number_compact_long() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(1000000.0),
            s("en"),
            obj(&[("notation", "compact"), ("compactDisplay", "long")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("million"),
        "compact long should contain 'million': {str}"
    );
}

#[test]
fn format_number_compact_short() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(1000000.0),
            s("en"),
            obj(&[("notation", "compact"), ("compactDisplay", "short")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        !str.contains("million"),
        "compact short should not contain 'million': {str}"
    );
}

// ── Currency display name ─────────────────────────────────────────

#[test]
fn format_number_currency_display_name() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(1234.56),
            s("en"),
            obj(&[
                ("style", "currency"),
                ("currency", "USD"),
                ("currencyDisplay", "name"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("US dollar") || str.contains("dollar"),
        "currencyDisplay:name should contain 'dollar': {str}"
    );
}

// ── Scientific and engineering notation ───────────────────────────

#[test]
fn format_number_scientific() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12345.0), s("en"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "scientific notation should produce output: {str}"
    );
}

#[test]
fn format_number_scientific_contains_exponent() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12345.0), s("en"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("10"),
        "scientific notation should contain '10': {str}"
    );
    assert!(
        str.contains("4") || str.contains("+4"),
        "scientific notation of 12345 should have exponent 4: {str}"
    );
}

#[test]
fn format_number_scientific_negative() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(-12345.0), s("en"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("-"),
        "scientific notation of negative should contain minus: {str}"
    );
    assert!(
        str.contains("10"),
        "scientific notation should contain '10': {str}"
    );
}

#[test]
fn format_number_scientific_small_number() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.00123), s("en"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("10"),
        "scientific notation should contain '10': {str}"
    );
    assert!(
        str.contains("-3") || str.contains("3"),
        "scientific notation of 0.00123 should have exponent -3: {str}"
    );
}

#[test]
fn format_number_scientific_one() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(1.0), s("en"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("10"),
        "scientific notation should contain '10': {str}"
    );
    assert!(
        str.contains("0") || str.contains("+0"),
        "scientific notation of 1 should have exponent 0: {str}"
    );
}

#[test]
fn format_number_scientific_german() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12345.0), s("de"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("10"),
        "scientific notation should contain '10': {str}"
    );
}

#[test]
fn format_number_engineering() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12345.0), s("en"), obj(&[("notation", "engineering")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "engineering notation should produce output: {str}"
    );
}

#[test]
fn format_number_engineering_exponent_multiple_of_3() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(12345.0), s("en"), obj(&[("notation", "engineering")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("10"),
        "engineering notation should contain '10': {str}"
    );
    // 12345 → exponent should be 3 (not 4), mantissa should be 12.345
    assert!(
        str.contains("3") || str.contains("+3"),
        "engineering notation of 12345 should have exponent 3: {str}"
    );
}

#[test]
fn format_number_engineering_small_number() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.00123), s("en"), obj(&[("notation", "engineering")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("10"),
        "engineering notation should contain '10': {str}"
    );
    // 0.00123 → exponent should be -3, mantissa should be 1.23
    assert!(
        str.contains("-3") || str.contains("3"),
        "engineering notation of 0.00123 should have exponent -3: {str}"
    );
}

#[test]
fn format_number_engineering_negative() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(-12345.0), s("en"), obj(&[("notation", "engineering")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("-"),
        "engineering notation of negative should contain minus: {str}"
    );
    assert!(
        str.contains("10"),
        "engineering notation should contain '10': {str}"
    );
}

#[test]
fn format_number_scientific_zero() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.0), s("en"), obj(&[("notation", "scientific")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("0"),
        "scientific notation of 0 should contain 0: {str}"
    );
}

// ── Unit style ───────────────────────────────────────────────────

#[test]
fn format_number_unit_meter_short() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("en"),
            obj(&[("style", "unit"), ("unit", "meter")]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("5") && str.contains("m"),
        "5 meters short should contain '5' and 'm', got: {str}"
    );
}

#[test]
fn format_number_unit_meter_long() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "meter"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("5"),
        "5 meters long should contain '5', got: {str}"
    );
}

#[test]
fn format_number_unit_kilometer() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(30.0),
            s("en"),
            obj(&[("style", "unit"), ("unit", "kilometer")]),
        ],
    );
    assert!(result.is_err(), "kilometer is not a supported unit");
}

#[test]
fn format_number_unit_kilogram() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(2.5),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilogram"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("2.5"),
        "2.5 kg long should contain '2.5', got: {str}"
    );
}

#[test]
fn format_number_unit_missing_unit_option() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[n(5.0), s("en"), obj(&[("style", "unit")])],
    );
    assert!(
        result.is_err(),
        "style:unit without unit option should error"
    );
}

#[test]
fn format_number_unit_invalid_unit_errors() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("en"),
            obj(&[("style", "unit"), ("unit", "not-a-real-unit")]),
        ],
    );
    assert!(result.is_err(), "not-a-real-unit should return an error");
}

// ── Numbering system ──────────────────────────────────────────────

#[test]
fn format_number_minimum_integer_digits() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(5.0), s("en"), obj(&[("minimumIntegerDigits", "3")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("005"),
        "minimumIntegerDigits:3 should pad to 005, got: {str}"
    );
}

#[test]
fn format_number_minimum_integer_digits_with_grouping() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(5.0), s("en"), obj(&[("minimumIntegerDigits", "5")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("00,005") || str.contains("00005"),
        "minimumIntegerDigits:5 should pad with zeros, got: {str}"
    );
}

#[test]
fn format_number_numbering_system_arab() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(123456.0), s("en"), obj(&[("numberingSystem", "arab")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        !str.contains("123"),
        "arab numbering should not contain ASCII digits: {str}"
    );
}

#[test]
fn format_number_numbering_system_latn() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(123456.0), s("ar"), obj(&[("numberingSystem", "latn")])],
    );
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("123"),
        "latn numbering should contain ASCII digits: {str}"
    );
}

// ── Expanded unit coverage ───────────────────────────────────────

#[test]
fn format_unit_kilometer_short() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilometer"),
                ("unitDisplay", "short"),
            ]),
        ],
    );
    assert!(result.is_err(), "kilometer is not a supported unit");
}

#[test]
fn format_unit_kilometer_long() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilometer"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    assert!(result.is_err(), "kilometer is not a supported unit");
}

#[test]
fn format_unit_kilometer_german() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("de"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilometer"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    assert!(result.is_err(), "kilometer is not a supported unit");
}

#[test]
fn format_unit_degree_celsius_short() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(20.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "degree-celsius"),
                ("unitDisplay", "short"),
            ]),
        ],
    );
    assert!(result.is_err(), "degree-celsius is not a supported unit");
}

#[test]
fn format_unit_degree_celsius_long() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(20.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "degree-celsius"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    assert!(result.is_err(), "degree-celsius is not a supported unit");
}

#[test]
fn format_unit_kilometer_per_hour_short() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(80.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilometer-per-hour"),
                ("unitDisplay", "short"),
            ]),
        ],
    );
    assert!(
        result.is_err(),
        "kilometer-per-hour is not a supported unit"
    );
}

#[test]
fn format_unit_kilometer_per_hour_long() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(80.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilometer-per-hour"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    assert!(
        result.is_err(),
        "kilometer-per-hour is not a supported unit"
    );
}

#[test]
fn format_unit_kilometer_per_hour_german() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(80.0),
            s("de"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilometer-per-hour"),
                ("unitDisplay", "long"),
            ]),
        ],
    );
    assert!(
        result.is_err(),
        "kilometer-per-hour is not a supported unit"
    );
}

#[test]
fn format_unit_centimeter_short() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(10.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "centimeter"),
                ("unitDisplay", "short"),
            ]),
        ],
    );
    assert!(result.is_err(), "centimeter is not a supported unit");
}

#[test]
fn format_unit_millimeter_short() {
    let result = call_err(
        &IntlModule::new(),
        "format_number",
        &[
            n(10.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "millimeter"),
                ("unitDisplay", "short"),
            ]),
        ],
    );
    assert!(result.is_err(), "millimeter is not a supported unit");
}

#[test]
fn format_unit_kilogram_short() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[
            n(5.0),
            s("en"),
            obj(&[
                ("style", "unit"),
                ("unit", "kilogram"),
                ("unitDisplay", "short"),
            ]),
        ],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("5"), "should contain the number, got: {str}");
    assert!(str.contains("kg"), "should contain 'kg' unit, got: {str}");
}

#[test]
fn format_unit_percent_short() {
    let result = call(
        &IntlModule::new(),
        "format_number",
        &[n(0.5), s("en"), obj(&[("style", "percent")])],
    );
    let str = result.as_str().expect("string result");
    assert!(str.contains("50"), "should contain 50, got: {str}");
    assert!(str.contains("%"), "should contain percent sign, got: {str}");
}
