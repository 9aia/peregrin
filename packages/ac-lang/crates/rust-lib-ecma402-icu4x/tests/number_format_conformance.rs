use rust_lib_ecma402_icu4x::number_format::NumberFormat;
use ecma402_traits::number_format::{
    CompactDisplay, CurrencyDisplay, CurrencySign, Notation, NumberFormat as _,
    NumberFormatOptions, NumberFormatPart, NumberFormatStyle, RoundingMode, SignDisplay,
    UnitDisplay, UseGrouping};
use serde::Deserialize;
use std::fs;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/conformance/fixtures/number_format.json"
);

#[derive(Deserialize, Debug)]
struct Fixture {
    cases: Vec<FixtureCase>}

#[derive(Deserialize, Debug)]
struct FixtureCase {
    id: String,
    locale: String,
    options: serde_json::Value,
    outputs: Vec<FixtureOutput>}

#[derive(Deserialize, Debug)]
struct FixtureOutput {
    input: serde_json::Value,
    formatted: String,
    parts: Vec<ExpectedPart>}

#[derive(Deserialize, Debug, PartialEq)]
struct ExpectedPart {
    #[serde(rename = "type")]
    part_type: String,
    value: String}

fn resolve_input(value: serde_json::Value) -> f64 {
    match value {
        serde_json::Value::Number(n) => n.as_f64().unwrap(),
        serde_json::Value::String(s) => match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            _ => panic!("unknown input literal: {s}")},
        _ => panic!("unsupported input: {value}")}
}

fn parse_style(value: &str) -> NumberFormatStyle {
    match value {
        "decimal" => NumberFormatStyle::Decimal,
        "currency" => NumberFormatStyle::Currency,
        "percent" => NumberFormatStyle::Percent,
        "unit" => NumberFormatStyle::Unit,
        _ => panic!("unknown style: {value}")}
}

fn parse_currency_display(value: &str) -> CurrencyDisplay {
    match value {
        "symbol" => CurrencyDisplay::Symbol,
        "narrowSymbol" => CurrencyDisplay::NarrowSymbol,
        "code" => CurrencyDisplay::Code,
        "name" => CurrencyDisplay::Name,
        _ => panic!("unknown currencyDisplay: {value}")}
}

fn parse_currency_sign(value: &str) -> CurrencySign {
    match value {
        "standard" => CurrencySign::Standard,
        "accounting" => CurrencySign::Accounting,
        _ => panic!("unknown currencySign: {value}")}
}

fn parse_notation(value: &str) -> Notation {
    match value {
        "standard" => Notation::Standard,
        "scientific" => Notation::Scientific,
        "engineering" => Notation::Engineering,
        "compact" => Notation::Compact,
        _ => panic!("unknown notation: {value}")}
}

fn parse_compact_display(value: &str) -> CompactDisplay {
    match value {
        "short" => CompactDisplay::Short,
        "long" => CompactDisplay::Long,
        _ => panic!("unknown compactDisplay: {value}")}
}

fn parse_use_grouping(value: &str) -> UseGrouping {
    match value {
        "true" => UseGrouping::True,
        "false" => UseGrouping::False,
        "auto" => UseGrouping::Auto,
        "always" => UseGrouping::Always,
        "min2" => UseGrouping::Min2,
        "never" => UseGrouping::Never,
        _ => panic!("unknown useGrouping: {value}")}
}

fn parse_unit_display(value: &str) -> UnitDisplay {
    match value {
        "short" => UnitDisplay::Short,
        "narrow" => UnitDisplay::Narrow,
        "long" => UnitDisplay::Long,
        _ => panic!("unknown unitDisplay: {value}")}
}

fn parse_sign_display(value: &str) -> SignDisplay {
    match value {
        "auto" => SignDisplay::Auto,
        "always" => SignDisplay::Always,
        "exceptZero" => SignDisplay::ExceptZero,
        "negative" => SignDisplay::Negative,
        "never" => SignDisplay::Never,
        _ => panic!("unknown signDisplay: {value}")}
}

fn parse_rounding_mode(value: &str) -> RoundingMode {
    match value {
        "ceil" => RoundingMode::Ceil,
        "floor" => RoundingMode::Floor,
        "expand" => RoundingMode::Expand,
        "trunc" => RoundingMode::Trunc,
        "halfCeil" => RoundingMode::HalfCeil,
        "halfFloor" => RoundingMode::HalfFloor,
        "halfExpand" => RoundingMode::HalfExpand,
        "halfTrunc" => RoundingMode::HalfTrunc,
        "halfEven" => RoundingMode::HalfEven,
        _ => panic!("unknown roundingMode: {value}")}
}

fn parse_options(raw: &serde_json::Value) -> NumberFormatOptions {
    let mut options = NumberFormatOptions::default();
    let Some(obj) = raw.as_object() else {
        return options;
    };

    if let Some(v) = obj.get("style").and_then(|v| v.as_str()) {
        options.style = parse_style(v);
    }
    if let Some(v) = obj.get("currency").and_then(|v| v.as_str()) {
        options.currency = Some(v.to_string());
    }
    if let Some(v) = obj.get("currencyDisplay").and_then(|v| v.as_str()) {
        options.currency_display = parse_currency_display(v);
    }
    if let Some(v) = obj.get("currencySign").and_then(|v| v.as_str()) {
        options.currency_sign = parse_currency_sign(v);
    }
    if let Some(v) = obj.get("notation").and_then(|v| v.as_str()) {
        options.notation = parse_notation(v);
    }
    if let Some(v) = obj.get("compactDisplay").and_then(|v| v.as_str()) {
        options.compact_display = parse_compact_display(v);
    }
    if let Some(v) = obj.get("minimumIntegerDigits").and_then(|v| v.as_i64()) {
        options.minimum_integer_digits = Some(v);
    }
    if let Some(v) = obj.get("minimumFractionDigits").and_then(|v| v.as_i64()) {
        options.minimum_fraction_digits = Some(v);
    }
    if let Some(v) = obj.get("maximumFractionDigits").and_then(|v| v.as_i64()) {
        options.maximum_fraction_digits = Some(v);
    }
    if let Some(v) = obj.get("minimumSignificantDigits").and_then(|v| v.as_i64()) {
        options.minimum_significant_digits = Some(v);
    }
    if let Some(v) = obj.get("maximumSignificantDigits").and_then(|v| v.as_i64()) {
        options.maximum_significant_digits = Some(v);
    }
    if let Some(v) = obj.get("useGrouping").and_then(|v| v.as_str()) {
        options.use_grouping = parse_use_grouping(v);
    } else if let Some(v) = obj.get("useGrouping").and_then(|v| v.as_bool()) {
        options.use_grouping = if v {
            UseGrouping::True
        } else {
            UseGrouping::False
        };
    }
    if let Some(v) = obj.get("unit").and_then(|v| v.as_str()) {
        options.unit = Some(v.to_string());
    }
    if let Some(v) = obj.get("unitDisplay").and_then(|v| v.as_str()) {
        options.unit_display = parse_unit_display(v);
    }
    if let Some(v) = obj.get("signDisplay").and_then(|v| v.as_str()) {
        options.sign_display = parse_sign_display(v);
    }
    if let Some(v) = obj.get("roundingMode").and_then(|v| v.as_str()) {
        options.rounding_mode = parse_rounding_mode(v);
    }
    if let Some(v) = obj.get("numberingSystem").and_then(|v| v.as_str()) {
        options.numbering_system = Some(v.to_string());
    }

    options
}

fn actual_to_expected(parts: &[NumberFormatPart]) -> Vec<ExpectedPart> {
    parts
        .iter()
        .map(|p| ExpectedPart {
            part_type: p.part_type.as_str().to_string(),
            value: p.value.clone()})
        .collect()
}

#[test]
fn number_format_conformance() {
    let fixture_json = fs::read_to_string(FIXTURE)
        .unwrap_or_else(|e| panic!("could not read fixture {FIXTURE}: {e}"));
    let fixture: Fixture = serde_json::from_str(&fixture_json)
        .unwrap_or_else(|e| panic!("could not parse fixture: {e}"));

    let mut failures = Vec::new();

    for case in &fixture.cases {
        let options = parse_options(&case.options);
        let formatter =
            NumberFormat::new(&case.locale, options).unwrap_or_else(|e| panic!("{case:?}: {e}"));

        for output in &case.outputs {
            let value = resolve_input(output.input.clone());
            let formatted = formatter
                .format(value)
                .unwrap_or_else(|e| panic!("{case:?}: {e}"));
            if formatted != output.formatted {
                failures.push(format!(
                    "{} ({}): formatted mismatch: got {:?}, expected {:?}",
                    case.id, value, formatted, output.formatted
                ));
            }

            let parts = formatter
                .format_to_parts(value)
                .unwrap_or_else(|e| panic!("{case:?}: {e}"));
            let actual_parts = actual_to_expected(&parts);
            if actual_parts != output.parts {
                failures.push(format!(
                    "{} ({}): parts mismatch:\n  got      {:?}\n  expected {:?}",
                    case.id, value, actual_parts, output.parts
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!("conformance failures:\n{}", failures.join("\n"));
    }
}
