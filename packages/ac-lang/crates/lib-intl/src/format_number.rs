use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{optional_locale, require_number, s};
use crate::locale::Locale;

/// `intl.format_number(n: number, locale?: string) → string`
///
/// Formats a number using locale-aware decimal and thousands separators.
///
/// Examples:
/// - `intl.format_number(1234567.89)` → `"1,234,567.89"`
/// - `intl.format_number(1234567.89, "pt-BR")` → `"1.234.567,89"`
/// - `intl.format_number(42)` → `"42"`
#[derive(Clone, Copy)]
pub struct FormatNumber;

impl Function for FormatNumber {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let data = locale.data();

        let formatted = format_number_impl(n, data.decimal_sep, data.thousands_sep);
        Ok(s(formatted))
    }
}

/// Format a number with locale-aware separators.
/// Handles grouping in 3-digit groups (Western convention).
fn format_number_impl(n: f64, decimal_sep: &str, thousands_sep: &str) -> String {
    let negative = n < 0.0;
    let abs_n = n.abs();

    let int_part = abs_n.floor() as i64;
    let frac_part = abs_n - int_part as f64;

    let int_str = int_part.to_string();
    let grouped = group_digits(&int_str, thousands_sep);

    let result = if frac_part > 0.0 {
        let frac_str = format!("{:.10}", frac_part);
        let frac_trimmed = frac_str
            .trim_start_matches("0.")
            .trim_end_matches('0');
        if frac_trimmed.is_empty() {
            grouped
        } else {
            format!("{}{}{}", grouped, decimal_sep, frac_trimmed)
        }
    } else {
        grouped
    };

    if negative {
        format!("-{result}")
    } else {
        result
    }
}

/// Group digits in 3s from the right, inserting the thousands separator.
fn group_digits(digits: &str, sep: &str) -> String {
    let len = digits.len();
    if len <= 3 {
        return digits.to_string();
    }

    let mut result = String::with_capacity(len + (len / 3));
    let first_group_len = len % 3;
    let mut idx = 0;

    if first_group_len > 0 {
        result.push_str(&digits[..first_group_len]);
        idx = first_group_len;
    }

    while idx < len {
        if !result.is_empty() {
            result.push_str(sep);
        }
        result.push_str(&digits[idx..idx + 3]);
        idx += 3;
    }

    result
}
