use ac_runtime_modules::Module;
use ac_runtime_values::Context;
use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{CanonicalContext, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("format_date").expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("format_date").expect("function exists");
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
fn format_date_basic() {
    let result = call(&[s("2024-05-08")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2024") && s.contains("8"));
}

#[test]
fn format_date_german() {
    let result = call(&[s("2024-05-08"), s("de")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2024") && s.contains("5") && s.contains("8"));
}

#[test]
fn format_date_short() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("style", "short")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("24") || s.contains("2024"));
}

#[test]
fn format_date_long() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("style", "long")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("May") || s.contains("2024"));
}

#[test]
fn format_datetime() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "datetime")]),
    ]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2024") && s.contains("30"));
}

#[test]
fn format_datetime_short() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "datetime"), ("style", "short")]),
    ]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

#[test]
fn format_date_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn format_date_invalid_format_errors() {
    assert!(call_err(&[s("not-a-date")]).is_err());
}

#[test]
fn format_date_invalid_date_errors() {
    assert!(call_err(&[s("2024-13-45")]).is_err());
}

#[test]
fn format_date_japanese_locale() {
    let result = call(&[s("2024-05-08"), s("ja")]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
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
fn vm_format_date() {
    let expr = Expr::call("intl.format_date", vec![Expr::value(s("2024-05-08"))]);
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}

#[test]
fn format_date_time_only() {
    let result = call(&[s("14:30:00"), s("en"), obj(&[("type", "time")])]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2:30") || str.contains("14:30"));
}

#[test]
fn format_date_time_only_short() {
    let result = call(&[
        s("14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("style", "short")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn format_date_datetime_with_timezone() {
    let result = call(&[
        s("2024-05-08T14:30:00+05:30"),
        s("en"),
        obj(&[("type", "datetime")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"));
}

#[test]
fn format_date_datetime_with_z_timezone() {
    let result = call(&[
        s("2024-05-08T14:30:00Z"),
        s("en"),
        obj(&[("type", "datetime")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"));
}

#[test]
fn format_date_datetime_with_fractional_seconds() {
    let result = call(&[
        s("2024-05-08T14:30:00.123456"),
        s("en"),
        obj(&[("type", "datetime")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"));
}

#[test]
fn format_date_negative_year() {
    let result = call(&[s("-0044-03-15"), s("en")]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

// ── hourCycle ────────────────────────────────────────────────────

#[test]
fn format_date_hour_cycle_h23() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("hourCycle", "h23")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("14:30"), "got: {str}");
}

#[test]
fn format_date_hour_cycle_h12() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("hourCycle", "h12")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2:30") || str.contains("PM"), "got: {str}");
}

// ── hour12 ───────────────────────────────────────────────────────

#[test]
fn format_date_hour12_true() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("hour12", "true")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("2:30") || str.contains("PM"),
        "hour12:true should use 12-hour, got: {str}"
    );
}

#[test]
fn format_date_hour12_false() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("hour12", "false")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("14:30"),
        "hour12:false should use 24-hour, got: {str}"
    );
}

#[test]
fn format_date_hour_cycle_overrides_hour12() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("hour12", "false"), ("hourCycle", "h12")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("2:30") || str.contains("PM"),
        "hourCycle should override hour12, got: {str}"
    );
}

// ── calendar ─────────────────────────────────────────────────────

#[test]
fn format_date_buddhist_calendar() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("calendar", "buddhist")])]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("2567"),
        "buddhist year should be 2567, got: {str}"
    );
}

#[test]
fn format_date_japanese_calendar() {
    let result = call(&[s("2024-05-08"), s("ja"), obj(&[("calendar", "japanese")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

#[test]
fn format_date_hebrew_calendar() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("calendar", "hebrew")])]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty());
}

// ── fractionalSecondDigits ───────────────────────────────────────

#[test]
fn format_date_fractional_seconds_3() {
    let result = call(&[
        s("2024-05-08T14:30:00.123"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("123"), "got: {str}");
}

#[test]
fn format_time_fractional_seconds_1() {
    let result = call(&[
        s("14:30:00.1"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "1")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("1"), "got: {str}");
}

#[test]
fn format_time_fractional_seconds_2() {
    let result = call(&[
        s("14:30:00.12"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "2")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("12"), "got: {str}");
}

#[test]
fn format_time_fractional_seconds_pads_single_digit() {
    let result = call(&[
        s("14:30:00.1"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("100"),
        "single digit .1 should pad to .100, got: {str}"
    );
}

#[test]
fn format_time_fractional_seconds_truncates_6_digits() {
    let result = call(&[
        s("14:30:00.123456"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("123"), "got: {str}");
}

#[test]
fn format_datetime_fractional_seconds_3() {
    let result = call(&[
        s("2024-05-08T14:30:00.123"),
        s("en"),
        obj(&[("type", "datetime"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("123"), "got: {str}");
}

#[test]
fn format_datetime_fractional_seconds_with_z() {
    let result = call(&[
        s("2024-05-08T14:30:00.123Z"),
        s("en"),
        obj(&[("type", "datetime"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("123"), "got: {str}");
}

#[test]
fn format_datetime_fractional_seconds_with_offset() {
    let result = call(&[
        s("2024-05-08T14:30:00.123+05:30"),
        s("en"),
        obj(&[("type", "datetime"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("123"), "got: {str}");
}

#[test]
fn format_time_fractional_seconds_with_z() {
    let result = call(&[
        s("14:30:00.123Z"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("123"), "got: {str}");
}

#[test]
fn format_time_fractional_seconds_no_digits_option() {
    let result = call(&[s("14:30:00.123"), s("en"), obj(&[("type", "time")])]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.contains("123"),
        "without fractionalSecondDigits, subseconds should not appear, got: {str}"
    );
}

#[test]
fn format_time_fractional_seconds_9_digits() {
    let result = call(&[
        s("14:30:00.123456789"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("123"),
        "9-digit fractional should truncate to 3 for display, got: {str}"
    );
}

#[test]
fn format_time_no_fractional_seconds_with_digits_option() {
    let result = call(&[
        s("14:30:00"),
        s("en"),
        obj(&[("type", "time"), ("fractionalSecondDigits", "3")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("000"),
        "no fractional seconds with digits=3 should show .000, got: {str}"
    );
}

// ── ECMA-402 field bag options ───────────────────────────────────

#[test]
fn field_bag_year_month_day() {
    let result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[("year", "numeric"), ("month", "long"), ("day", "numeric")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"), "should contain year, got: {str}");
    assert!(str.contains("May"), "should contain month name, got: {str}");
    assert!(str.contains("8"), "should contain day, got: {str}");
}

#[test]
fn field_bag_weekday_only() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("weekday", "long")])]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("Wednesday"),
        "should contain weekday, got: {str}"
    );
}

#[test]
fn field_bag_time_fields() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("hour", "numeric"), ("minute", "2-digit")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty(), "should produce output, got: {str}");
}

#[test]
fn field_bag_date_and_time() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[
            ("year", "numeric"),
            ("month", "short"),
            ("day", "numeric"),
            ("hour", "numeric"),
            ("minute", "2-digit"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"), "should contain year, got: {str}");
    assert!(str.contains("May"), "should contain month, got: {str}");
}

#[test]
fn field_bag_era() {
    let result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[
            ("year", "numeric"),
            ("month", "long"),
            ("day", "numeric"),
            ("era", "long"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"), "should contain year, got: {str}");
    assert!(str.contains("May"), "should contain month, got: {str}");
}

#[test]
fn field_bag_month_day() {
    let result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[("month", "long"), ("day", "numeric")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("May"), "should contain month, got: {str}");
    assert!(str.contains("8"), "should contain day, got: {str}");
}

#[test]
fn field_bag_time_only_from_time_string() {
    let result = call(&[
        s("14:30:00"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("second", "2-digit"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(!str.is_empty(), "should produce output, got: {str}");
}

#[test]
fn field_bag_overrides_style_and_type() {
    let result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[
            ("style", "short"),
            ("type", "time"),
            ("year", "long"),
            ("month", "long"),
            ("day", "numeric"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("2024"),
        "field bag should override style+type, got: {str}"
    );
    assert!(
        str.contains("May"),
        "field bag with month:long should use long month, got: {str}"
    );
}

// ── Timezone support ─────────────────────────────────────────────

#[test]
fn format_date_timezone_short_offset() {
    let result = call(&[
        s("2024-05-08T14:30:00+05:30"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "shortOffset"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("GMT") || str.contains("+5") || str.contains("+05"),
        "shortOffset should contain offset info, got: {str}"
    );
}

#[test]
fn format_date_timezone_long_offset() {
    let result = call(&[
        s("2024-05-08T14:30:00+05:30"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "longOffset"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("GMT") || str.contains("+5") || str.contains("+05"),
        "longOffset should contain offset info, got: {str}"
    );
}

#[test]
fn format_date_timezone_zulu() {
    let result = call(&[
        s("2024-05-08T14:30:00Z"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "shortOffset"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("GMT") || str.contains("Z") || str.contains("+00"),
        "Z timezone with shortOffset should contain GMT or +00, got: {str}"
    );
}

#[test]
fn format_date_timezone_negative_offset() {
    let result = call(&[
        s("2024-05-08T14:30:00-08:00"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "shortOffset"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("GMT") || str.contains("-8") || str.contains("-08"),
        "negative offset should contain -8 or -08, got: {str}"
    );
}

#[test]
fn format_date_timezone_with_date_fields() {
    let result = call(&[
        s("2024-05-08T14:30:00+05:30"),
        s("en"),
        obj(&[
            ("year", "numeric"),
            ("month", "long"),
            ("day", "numeric"),
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "shortOffset"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(str.contains("2024"), "should contain year, got: {str}");
    assert!(
        str.contains("GMT") || str.contains("+5") || str.contains("+05"),
        "should contain timezone offset, got: {str}"
    );
}

#[test]
fn format_date_timezone_no_offset_in_string() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "shortOffset"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("GMT") || str.contains("+00"),
        "no offset should default to UTC/GMT, got: {str}"
    );
}
