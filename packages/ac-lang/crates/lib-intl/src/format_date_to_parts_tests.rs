use ac_runtime_modules::Module;
use ac_runtime_values::Context;
use ac_runtime_values::Error;
use ac_runtime_values::NonFunctionValue;

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module
        .function("format_date_to_parts")
        .expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module
        .function("format_date_to_parts")
        .expect("function exists");
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

fn parts_to_string(parts: &[(String, String)]) -> String {
    parts.iter().map(|(_, v)| v.as_str()).collect()
}

// ── Basic date ───────────────────────────────────────────────────

#[test]
fn date_to_parts_basic() {
    let result = call(&[s("2024-05-08")]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"integer") || types.contains(&"month") || types.contains(&"literal"),
        "date parts should contain integer/month/literal, got: {types:?}"
    );
}

#[test]
fn date_to_parts_year() {
    let result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[("year", "numeric"), ("month", "long"), ("day", "numeric")]),
    ]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"integer"),
        "should have integer part for year, got: {types:?}"
    );
    assert!(
        types.contains(&"month"),
        "should have month part, got: {types:?}"
    );
}

#[test]
fn date_to_parts_month_name() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("month", "long")])]);
    let parts = extract_parts(result);
    let combined = parts_to_string(&parts);
    assert!(
        combined.contains("May"),
        "should contain month name 'May', got: {combined}"
    );
}

#[test]
fn date_to_parts_weekday() {
    let result = call(&[s("2024-05-08"), s("en"), obj(&[("weekday", "long")])]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"weekday"),
        "should have weekday part, got: {types:?}"
    );
    let combined = parts_to_string(&parts);
    assert!(
        combined.contains("Wednesday"),
        "should contain 'Wednesday', got: {combined}"
    );
}

// ── Time ─────────────────────────────────────────────────────────

#[test]
fn date_to_parts_time() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("second", "2-digit"),
        ]),
    ]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"integer"),
        "should have integer part for hour, got: {types:?}"
    );
    assert!(
        types.contains(&"dayPeriod"),
        "should have dayPeriod part, got: {types:?}"
    );
}

#[test]
fn date_to_parts_time_only() {
    let result = call(&[s("14:30:00"), s("en"), obj(&[("type", "time")])]);
    let parts = extract_parts(result);
    let combined = parts_to_string(&parts);
    assert!(
        combined.contains("30"),
        "should contain minutes, got: {combined}"
    );
}

// ── Datetime ─────────────────────────────────────────────────────

#[test]
fn date_to_parts_datetime() {
    let result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "datetime")]),
    ]);
    let parts = extract_parts(result);
    let combined = parts_to_string(&parts);
    assert!(
        combined.contains("2024"),
        "should contain year, got: {combined}"
    );
    assert!(
        combined.contains("30"),
        "should contain minutes, got: {combined}"
    );
}

// ── Literal parts ────────────────────────────────────────────────

#[test]
fn date_to_parts_has_literals() {
    let result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[("year", "numeric"), ("month", "long"), ("day", "numeric")]),
    ]);
    let parts = extract_parts(result);
    let types: Vec<&str> = parts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        types.contains(&"literal"),
        "should have literal parts between fields, got: {types:?}"
    );
}

// ── Error cases ──────────────────────────────────────────────────

#[test]
fn date_to_parts_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn date_to_parts_invalid_date_errors() {
    assert!(call_err(&[s("not-a-date")]).is_err());
}

// ── Consistency: parts concatenated == format_date string ────────

#[test]
fn date_to_parts_matches_format_date() {
    let module = IntlModule::new();
    let formatted = module
        .function("format_date")
        .unwrap()
        .call(
            &[
                s("2024-05-08"),
                s("en"),
                obj(&[("year", "numeric"), ("month", "long"), ("day", "numeric")]),
            ],
            &mut CanonicalContext::new(),
        )
        .expect("format_date succeeds");
    let parts_result = call(&[
        s("2024-05-08"),
        s("en"),
        obj(&[("year", "numeric"), ("month", "long"), ("day", "numeric")]),
    ]);
    let parts_str = parts_to_string(&extract_parts(parts_result));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_date output"
    );
}

#[test]
fn date_to_parts_matches_format_date_datetime() {
    let module = IntlModule::new();
    let formatted = module
        .function("format_date")
        .unwrap()
        .call(
            &[
                s("2024-05-08T14:30:00"),
                s("en"),
                obj(&[("type", "datetime")]),
            ],
            &mut CanonicalContext::new(),
        )
        .expect("format_date succeeds");
    let parts_result = call(&[
        s("2024-05-08T14:30:00"),
        s("en"),
        obj(&[("type", "datetime")]),
    ]);
    let parts_str = parts_to_string(&extract_parts(parts_result));
    assert_eq!(
        formatted.as_str(),
        Some(parts_str.as_str()),
        "concatenated parts should match format_date output for datetime"
    );
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::{CanonicalContext, Host, Vm};
use ac_runtime_modules::ModuleHost;

#[test]
fn vm_format_date_to_parts() {
    let expr = Expr::call(
        "intl.format_date_to_parts",
        vec![Expr::value(s("2024-05-08"))],
    );
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    let result = vm.eval(&expr, &mut ctx).expect("eval succeeds");
    assert!(result.is_array(), "result should be an array");
}

#[test]
fn vm_resolves_format_date_to_parts() {
    let host = ModuleHost::new().with(IntlModule::new());
    assert!(host.function("intl.format_date_to_parts").is_some());
}

// ── Timezone name in parts ───────────────────────────────────────

#[test]
fn date_to_parts_with_timezone_name() {
    let result = call(&[
        s("2024-05-08T14:30:00+05:30"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "longOffset"),
        ]),
    ]);
    let parts = extract_parts(result);
    let combined = parts_to_string(&parts);
    assert!(
        combined.contains("GMT") || combined.contains("+5") || combined.contains("+05"),
        "longOffset should contain offset info, got: {combined}"
    );
}

#[test]
fn date_to_parts_with_timezone_short_offset() {
    let result = call(&[
        s("2024-05-08T14:30:00+05:30"),
        s("en"),
        obj(&[
            ("hour", "numeric"),
            ("minute", "2-digit"),
            ("timeZoneName", "shortOffset"),
        ]),
    ]);
    let parts = extract_parts(result);
    let combined = parts_to_string(&parts);
    assert!(
        combined.contains("GMT") || combined.contains("+5") || combined.contains("+05"),
        "shortOffset should contain offset info, got: {combined}"
    );
}
