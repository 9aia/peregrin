use ac_runtime_modules::Module;
use ac_runtime_values::Context;
use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{CanonicalContext, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("format_duration").expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("format_duration").expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn obj(pairs: &[(&str, u64)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

fn opts(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

#[test]
fn format_duration_seconds() {
    let result = call(&[obj(&[("seconds", 90)])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("90") && s.contains("second"));
}

#[test]
fn format_duration_hours_minutes() {
    let result = call(&[obj(&[("hours", 1), ("minutes", 30)])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("1") && s.contains("30"));
}

#[test]
fn format_duration_german() {
    let result = call(&[obj(&[("seconds", 90)]), s("de")]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("90") && s.contains("Sekunde"));
}

#[test]
fn format_duration_short_style() {
    let result = call(&[
        obj(&[("hours", 2), ("minutes", 5)]),
        s("en"),
        opts(&[("style", "short")]),
    ]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2") && s.contains("5"));
}

#[test]
fn format_duration_narrow_style() {
    let result = call(&[obj(&[("hours", 2)]), s("en"), opts(&[("style", "narrow")])]);
    let s = result.as_str().expect("string result");
    assert!(s.contains("2"));
}

#[test]
fn format_duration_digital_style() {
    let result = call(&[
        obj(&[("hours", 1), ("minutes", 30)]),
        s("en"),
        opts(&[("style", "digital")]),
    ]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

#[test]
fn format_duration_empty() {
    let result = call(&[obj(&[])]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

#[test]
fn format_duration_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn format_duration_all_fields() {
    let result = call(&[obj(&[
        ("years", 1),
        ("months", 2),
        ("weeks", 3),
        ("days", 4),
        ("hours", 5),
        ("minutes", 6),
        ("seconds", 7),
    ])]);
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
fn vm_format_duration() {
    let expr = Expr::call(
        "intl.format_duration",
        vec![Expr::value(obj(&[("seconds", 90)]))],
    );
    let result = eval_expr(&expr);
    assert!(result.as_str().is_some());
}

#[test]
fn format_duration_negative() {
    let mut map = serde_json::Map::new();
    map.insert("seconds".to_string(), serde_json::json!(90));
    map.insert("negative".to_string(), serde_json::json!(true));
    let result = call(&[NonFunctionValue::new(serde_json::Value::Object(map))]);
    let s = result.as_str().expect("string result");
    assert!(!s.is_empty());
}

// ── Per-unit style and display options ────────────────────────────

fn obj_str(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

#[test]
fn duration_per_unit_hours_style_long() {
    let result = call(&[
        obj(&[("hours", 2), ("minutes", 30)]),
        s("en"),
        obj_str(&[("hoursStyle", "long")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("hour"),
        "hoursStyle:long should contain 'hour': {str}"
    );
}

#[test]
fn duration_per_unit_minutes_style_short() {
    let result = call(&[
        obj(&[("hours", 2), ("minutes", 30)]),
        s("en"),
        obj_str(&[("minutesStyle", "short")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("min"),
        "minutesStyle:short should contain 'min': {str}"
    );
}

#[test]
fn duration_per_unit_seconds_style_narrow() {
    let result = call(&[
        obj(&[("seconds", 45)]),
        s("en"),
        obj_str(&[("secondsStyle", "narrow")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "secondsStyle:narrow should produce output: {str}"
    );
}

#[test]
fn duration_per_unit_hours_display_always() {
    let result = call(&[
        obj(&[("hours", 0), ("minutes", 30)]),
        s("en"),
        obj_str(&[("hoursDisplay", "always")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "hoursDisplay:always should produce output: {str}"
    );
}

#[test]
fn duration_per_unit_years_style_long() {
    let result = call(&[
        obj(&[("years", 1), ("months", 6)]),
        s("en"),
        obj_str(&[("yearsStyle", "long")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("year"),
        "yearsStyle:long should contain 'year': {str}"
    );
}

#[test]
fn duration_per_unit_days_style_short() {
    let result = call(&[
        obj(&[("days", 3), ("hours", 4)]),
        s("en"),
        obj_str(&[("daysStyle", "short")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("day") || str.contains("d"),
        "daysStyle:short should contain day abbreviation: {str}"
    );
}

#[test]
fn duration_per_unit_milliseconds_style_numeric() {
    let result = call(&[
        obj(&[("seconds", 1), ("milliseconds", 500)]),
        s("en"),
        obj_str(&[("millisecondsStyle", "numeric")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "millisecondsStyle:numeric should produce output: {str}"
    );
}

#[test]
fn duration_per_unit_mixed_styles() {
    let result = call(&[
        obj(&[("hours", 2), ("minutes", 30), ("seconds", 45)]),
        s("en"),
        obj_str(&[
            ("hoursStyle", "long"),
            ("minutesStyle", "short"),
            ("secondsStyle", "narrow"),
        ]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("hour"),
        "should contain 'hour' from hoursStyle:long: {str}"
    );
    assert!(
        str.contains("min"),
        "should contain 'min' from minutesStyle:short: {str}"
    );
}

#[test]
fn duration_fractional_digits() {
    let result = call(&[
        obj(&[("seconds", 1), ("milliseconds", 500)]),
        s("en"),
        obj_str(&[("millisecondsStyle", "numeric"), ("fractionalDigits", "2")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "fractionalDigits should produce output: {str}"
    );
}

#[test]
fn duration_per_unit_weeks_style() {
    let result = call(&[
        obj(&[("weeks", 2), ("days", 3)]),
        s("en"),
        obj_str(&[("weeksStyle", "long")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        str.contains("week"),
        "weeksStyle:long should contain 'week': {str}"
    );
}

#[test]
fn duration_hours_numeric_style() {
    let result = call(&[
        obj(&[("hours", 2), ("minutes", 5)]),
        s("en"),
        obj_str(&[("hoursStyle", "numeric"), ("minutesStyle", "numeric")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "numeric style should produce output: {str}"
    );
}

#[test]
fn duration_seconds_two_digit_style() {
    let result = call(&[
        obj(&[("minutes", 1), ("seconds", 5)]),
        s("en"),
        obj_str(&[("secondsStyle", "2-digit")]),
    ]);
    let str = result.as_str().expect("string result");
    assert!(
        !str.is_empty(),
        "2-digit style should produce output: {str}"
    );
}
