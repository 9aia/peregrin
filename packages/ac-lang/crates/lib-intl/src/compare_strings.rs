use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::collator::{Collator, options::CollatorOptions, options::Strength};

use super::common::{opt_str, optional_locale, optional_object, require_string};

/// `intl.compare_strings(a: string, b: string, locale?: string, opts?: object) → number`
///
/// Compares two strings using locale-aware collation via ICU4X.
///
/// Returns `-1` if `a < b`, `0` if `a == b`, `1` if `a > b`.
///
/// Options:
/// - `sensitivity`: `"base"`, `"accent"`, `"case"`, `"variant"` (default)
///
/// Examples:
/// - `intl.compare_strings("a", "b")` → `-1`
/// - `intl.compare_strings("a", "a")` → `0`
/// - `intl.compare_strings("b", "a")` → `1`
/// - `intl.compare_strings("ä", "a", "de", {sensitivity: "base"})` → `0`
#[derive(Clone, Copy)]
pub struct CompareStrings;

impl Function for CompareStrings {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let a = require_string(args, 0)?;
        let b = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;

        let mut options = CollatorOptions::default();

        if let Some(sensitivity) = opt_str(opts, "sensitivity") {
            options.strength = Some(match sensitivity {
                "base" => Strength::Primary,
                "accent" => Strength::Secondary,
                "case" => Strength::Tertiary,
                _ => Strength::Quaternary,
            });
        }

        let collator = Collator::try_new(locale.into(), options)
            .map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;

        let result = collator.compare(a, b);
        let value = match result {
            core::cmp::Ordering::Less => -1.0,
            core::cmp::Ordering::Equal => 0.0,
            core::cmp::Ordering::Greater => 1.0,
        };

        Ok(NonFunctionValue::new(serde_json::json!(value)))
    }
}
