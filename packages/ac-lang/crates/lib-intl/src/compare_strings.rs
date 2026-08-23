use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use icu::collator::{Collator, CollatorPreferences,
    options::{AlternateHandling, CollatorOptions, MaxVariable, Strength, CaseLevel},
    preferences::{CollationCaseFirst, CollationNumericOrdering}};
use std::cmp::Ordering;

use super::common::{make_provider, opt_bool, opt_str, optional_locale, optional_object, require_string};
use super::error::IntlError;
use super::provider::IntlProvider;

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
/// - `usage`: `"sort"` (default) or `"search"` — selects sort vs search collation
/// - `collation`: collation variant string (e.g. `"phonebk"` for German phonebook)
///
/// Examples:
/// - `intl.compare_strings("a", "b")` → `-1`
/// - `intl.compare_strings("é", "e", "en", {sensitivity: "base"})` → `0`
/// - `intl.compare_strings("file10", "file2", "en", {numeric: true})` → `1`
#[derive(Clone)]
pub struct CompareStrings {
    provider: Option<IntlProvider>}

impl CompareStrings {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for CompareStrings {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for CompareStrings {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let a = require_string(args, 0)?;
        let b = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;
        let dp = make_provider(&self.provider)?;

        let sensitivity = opt_str(opts, "sensitivity").unwrap_or("variant");

        let mut options = CollatorOptions::default();
        options.strength = Some(match sensitivity {
            "base" => Strength::Primary,
            "accent" => Strength::Secondary,
            "case" => { options.case_level = Some(CaseLevel::On); Strength::Primary },
            _ => Strength::Tertiary});

        let mut prefs = CollatorPreferences::from(&locale);
        prefs.case_first = match opt_str(opts, "caseFirst") {
            Some("upper") => Some(CollationCaseFirst::Upper),
            Some("lower") => Some(CollationCaseFirst::Lower),
            Some("false") => Some(CollationCaseFirst::False),
            _ => None};
        prefs.numeric_ordering = match opt_bool(opts, "numeric") {
            Some(true) => Some(CollationNumericOrdering::True),
            Some(false) => Some(CollationNumericOrdering::False),
            _ => None};

        let usage = opt_str(opts, "usage").unwrap_or("sort");
        let collation = opt_str(opts, "collation");
        if usage == "search" || collation == Some("search") {
            prefs.collation_type = Some(icu::collator::preferences::CollationType::Search);
        } else if let Some(col) = collation {
            prefs.collation_type = match col {
                "phonebk" => Some(icu::collator::preferences::CollationType::Phonebk),
                "stroke" => Some(icu::collator::preferences::CollationType::Stroke),
                "trad" => Some(icu::collator::preferences::CollationType::Trad),
                "pinyin" => Some(icu::collator::preferences::CollationType::Pinyin),
                _ => None};
        }

        if let Some(true) = opt_bool(opts, "ignorePunctuation") {
            options.alternate_handling = Some(AlternateHandling::Shifted);
            options.max_variable = Some(MaxVariable::Punctuation);
        }

        let result: f64 = if let Some(p) = dp.as_buffer_provider() {
            let collator = Collator::try_new_with_buffer_provider(p, prefs, options)
                .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
            match collator.as_borrowed().compare(a, b) {
                Ordering::Less => -1.0,
                Ordering::Equal => 0.0,
                Ordering::Greater => 1.0}
        } else {
            let collator = Collator::try_new(prefs, options)
                .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
            match collator.compare(a, b) {
                Ordering::Less => -1.0,
                Ordering::Equal => 0.0,
                Ordering::Greater => 1.0}
        };

        Ok(NonFunctionValue::new(result))
    }
}
