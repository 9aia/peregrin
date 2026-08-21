use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{optional_locale, require_number, s};
use crate::locale::Locale;
use crate::plural::{plural_category, PluralCategory};

/// `intl.ordinal(n: number, locale?: string) → string`
///
/// Returns the ordinal form of a number using locale-aware suffixes.
///
/// Examples:
/// - `intl.ordinal(1)` → `"1st"`
/// - `intl.ordinal(2)` → `"2nd"`
/// - `intl.ordinal(3)` → `"3rd"`
/// - `intl.ordinal(11)` → `"11th"`
/// - `intl.ordinal(2, "pt")` → `"2º"`
#[derive(Clone, Copy)]
pub struct Ordinal;

impl Function for Ordinal {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let data = locale.data();

        let int_n = n.abs() as i64;
        let suffix = english_ordinal_suffix(int_n, &locale, &data.ordinal_suffixes);

        Ok(s(format!("{}{}", int_n, suffix)))
    }
}

/// English ordinal rules: 11th/12th/13th are special, then st/nd/rd/th.
/// For non-English locales, all suffixes are typically the same.
fn english_ordinal_suffix(n: i64, locale: &Locale, suffixes: &crate::data::OrdinalSuffixes) -> &'static str {
    if locale.language() != "en" {
        return suffixes.other;
    }

    let last_two = n % 100;
    if (11..=13).contains(&last_two) {
        return suffixes.other;
    }

    let last = n % 10;
    let category = match last {
        1 => PluralCategory::One,
        2 => PluralCategory::Two,
        3 => PluralCategory::Few,
        _ => PluralCategory::Other,
    };

    // Verify via plural rules for consistency
    let _ = plural_category(locale, n as f64);

    match category {
        PluralCategory::One => suffixes.one,
        PluralCategory::Two => suffixes.two,
        PluralCategory::Few => suffixes.few,
        _ => suffixes.other,
    }
}
