use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::plurals::{PluralCategory, PluralRules, PluralRulesOptions, PluralRuleType};

use super::common::{opt_str, optional_locale, optional_object, require_number, s};
use super::error::IntlError;

/// `intl.select_plural(n: number, locale?: string, opts?: object) → string`
///
/// Returns the CLDR plural category for a number in the given locale.
///
/// Options:
/// - `type`: `"cardinal"` (default) or `"ordinal"`
///
/// Examples:
/// - `intl.select_plural(1)` → `"one"`
/// - `intl.select_plural(5)` → `"other"`
/// - `intl.select_plural(0, "ar")` → `"zero"`
/// - `intl.select_plural(2, "en", {type: "ordinal"})` → `"two"`
/// - `intl.select_plural(11, "en", {type: "ordinal"})` → `"other"`
#[derive(Clone, Copy)]
pub struct SelectPlural;

impl Function for SelectPlural {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;

        let rule_type = match opt_str(opts, "type").unwrap_or("cardinal") {
            "ordinal" => PluralRuleType::Ordinal,
            _ => PluralRuleType::Cardinal,
        };

        let options = PluralRulesOptions::default().with_type(rule_type);
        let rules = PluralRules::try_new(locale.into(), options)
            .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

        let category = rules.category_for(n as usize);
        let name = match category {
            PluralCategory::Zero => "zero",
            PluralCategory::One => "one",
            PluralCategory::Two => "two",
            PluralCategory::Few => "few",
            PluralCategory::Many => "many",
            PluralCategory::Other => "other",
        };
        Ok(s(name.to_string()))
    }
}
