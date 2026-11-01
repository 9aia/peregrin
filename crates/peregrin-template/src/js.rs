//! JavaScript value coercion helpers — the engine speaks `serde_json`
//! internally but must print values the way TS `String(v)` /
//! `toStringSafe` did so rendered output stays identical.

use serde_json::{Number, Value};

/// `toStringSafe` from `engine.ts`: null → `""`, arrays join on `,`
/// (recursively, null elements → `""`), plain objects → `[object Object]`.
pub(crate) fn to_string_safe(value: &Value) -> String {
    js_display(value, "")
}

/// `String(v)` as used by the capability matcher for glob patterns:
/// same as [`to_string_safe`] except null → `"null"`.
pub(crate) fn js_string(value: &Value) -> String {
    js_display(value, "null")
}

fn js_display(value: &Value, null: &str) -> String {
    match value {
        Value::Null => null.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| js_display(item, ""))
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `String(number)`: integral values print without a fraction
/// (`5` not `5.0`); other finite values defer to serde/ryu's shortest
/// round-trip, which matches JS for typical magnitudes.
fn number_to_string(number: &Number) -> String {
    if let Some(int) = number.as_i64() {
        return int.to_string();
    }
    if let Some(uint) = number.as_u64() {
        return uint.to_string();
    }
    match number.as_f64() {
        Some(float) if float.is_finite() && float.fract() == 0.0 && in_i64_range(float) =>
        {
            #[allow(clippy::cast_possible_truncation)]
            (float as i64).to_string()
        }
        _ => number.to_string(),
    }
}

/// Largest |f| guaranteed to survive `f as i64` faithfully.
const MAX_EXACT_INT: f64 = 9_007_199_254_740_992.0; // 2^53

fn in_i64_range(float: f64) -> bool {
    float.abs() <= MAX_EXACT_INT
}

/// Wrap an `f64` result the way a JS `Number` would: integral values
/// become `Value::Number` integers so they stringify without `.0`;
/// non-finite values (JS `Infinity`/`NaN`) become their `String()` text
/// since JSON has no representation for them.
pub(crate) fn js_number_value(float: f64) -> Value {
    if float.is_finite() && float.fract() == 0.0 && in_i64_range(float) {
        #[allow(clippy::cast_possible_truncation)]
        return Value::from(float as i64);
    }
    Number::from_f64(float).map_or_else(
        || {
            Value::String(if float.is_sign_negative() {
                "-Infinity".to_string()
            } else {
                "Infinity".to_string()
            })
        },
        Value::Number,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stringification() {
        assert_eq!(to_string_safe(&Value::Null), "");
        assert_eq!(js_string(&Value::Null), "null");
        assert_eq!(to_string_safe(&json!(true)), "true");
        assert_eq!(to_string_safe(&json!(5)), "5");
        assert_eq!(to_string_safe(&json!(5.5)), "5.5");
        assert_eq!(to_string_safe(&json!("x")), "x");
        assert_eq!(to_string_safe(&json!({"a": 1})), "[object Object]");
        assert_eq!(
            to_string_safe(&json!([1, [2, 3], null, {"a": 1}])),
            "1,2,3,,[object Object]"
        );
        assert_eq!(js_number_value(4.0), json!(4));
        assert_eq!(js_number_value(0.5), json!(0.5));
        assert_eq!(js_number_value(f64::INFINITY), json!("Infinity"));
    }
}
