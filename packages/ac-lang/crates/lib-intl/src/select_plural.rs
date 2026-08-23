use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::plurals::{PluralCategory, PluralRules, PluralRulesOptions, PluralRuleType};

use super::common::{apply_fraction_digits, apply_minimum_integer_digits, apply_significant_digits, decimal_from_f64, make_provider, opt_str, optional_locale, optional_object, require_number, s};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.select_plural(n: number, locale?: string, opts?: object) → string`
///
/// Returns the CLDR plural category for a number in the given locale.
///
/// Options:
/// - `type`: `"cardinal"` (default) or `"ordinal"`
/// - `minimumIntegerDigits`: minimum integer digits
/// - `minimumFractionDigits`: minimum fraction digits to display
/// - `maximumFractionDigits`: maximum fraction digits to display
/// - `minimumSignificantDigits`: minimum significant digits
/// - `maximumSignificantDigits`: maximum significant digits
///
/// Examples:
/// - `intl.select_plural(1)` → `"one"`
/// - `intl.select_plural(5)` → `"other"`
/// - `intl.select_plural(0, "ar")` → `"zero"`
/// - `intl.select_plural(2, "en", {type: "ordinal"})` → `"two"`
/// - `intl.select_plural(11, "en", {type: "ordinal"})` → `"other"`
/// - `intl.select_plural(1.0, "en")` → `"other"` (trailing zero → not "one")
/// - `intl.select_plural(1, "en", {minimumFractionDigits: 2})` → `"other"` (padded to 1.00)
#[derive(Clone)]
pub struct SelectPlural {
    provider: Option<IntlProvider>}

impl SelectPlural {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for SelectPlural {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for SelectPlural {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        let dp = make_provider(&self.provider)?;

        let rule_type = match opt_str(opts, "type").unwrap_or("cardinal") {
            "ordinal" => PluralRuleType::Ordinal,
            _ => PluralRuleType::Cardinal};

        let options = PluralRulesOptions::default().with_type(rule_type);
        let rules = match dp.as_buffer_provider() {
            Some(p) => PluralRules::try_new_with_buffer_provider(p, locale.into(), options),
            None => PluralRules::try_new(locale.into(), options)}
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

        let mut decimal = decimal_from_f64(n);
        apply_minimum_integer_digits(&mut decimal, opts);
        apply_fraction_digits(&mut decimal, opts);
        apply_significant_digits(&mut decimal, opts);

        let category = rules.category_for(&decimal);
        let name = match category {
            PluralCategory::Zero => "zero",
            PluralCategory::One => "one",
            PluralCategory::Two => "two",
            PluralCategory::Few => "few",
            PluralCategory::Many => "many",
            PluralCategory::Other => "other"};
        Ok(s(name.to_string()))
    }
}
