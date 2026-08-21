use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::collator::{Collator, CollatorOptions, CollatorPreferences,
    options::{Strength, CaseLevel},
    preferences::{CollationCaseFirst, CollationNumericOrdering},
};
use std::cmp::Ordering;

use super::common::{opt_bool, opt_str, optional_locale, optional_object, require_string, s};
use super::error::IntlError;

/// `intl.compare_strings(a: string, b: string, locale?: string, opts?: object) → number`
///
/// Compares two strings using ICU4X collation.
///
/// Returns `-1` if `a < b`, `0` if `a == b`, `1` if `a > b`.
///
/// Options:
/// - `sensitivity`: `"base"`, `"accent"`, `"case"`, or `"variant"` (default)
/// - `caseFirst`: `"upper"`, `"lower"`, or `"false"` (default)
/// - `numeric`: `true` or `false` (default) — sorts digit sequences by numeric value
/// - `ignorePunctuation`: `true` or `false` (default)
///
/// Examples:
/// - `intl.compare_strings("a", "b")` → `-1`
/// - `intl.compare_strings("é", "e", "en", {sensitivity: "base"})` → `0`
/// - `intl.compare_strings("file10", "file2", "en", {numeric: true})` → `1`
#[derive(Clone, Copy)]
pub struct CompareStrings;

impl Function for CompareStrings {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let a = require_string(args, 0)?;
        let b = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;

        let sensitivity = opt_str(opts, "sensitivity").unwrap_or("variant");

        let mut options = CollatorOptions::default();
        options.strength = Some(match sensitivity {
            "base" => Strength::Primary,
            "accent" => Strength::Secondary,
            "case" => { options.case_level = Some(CaseLevel::On); Strength::Primary },
            _ => Strength::Tertiary,
        });

        let mut prefs = CollatorPreferences::default();
        prefs.case_first = match opt_str(opts, "caseFirst") {
            Some("upper") => Some(CollationCaseFirst::Upper),
            Some("lower") => Some(CollationCaseFirst::Lower),
            Some("false") => Some(CollationCaseFirst::False),
            _ => None,
        };
        prefs.numeric_ordering = match opt_bool(opts, "numeric") {
            Some(true) => Some(CollationNumericOrdering::True),
            Some(false) => Some(CollationNumericOrdering::False),
            _ => None,
        };

        let collator = Collator::try_new(prefs, options)
            .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

        let result = match collator.compare(a, b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };

        Ok(s(result.to_string()))
    }
}
