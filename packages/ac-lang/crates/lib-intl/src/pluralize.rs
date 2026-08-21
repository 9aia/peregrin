use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{optional_locale, require_number, require_object, s};
use crate::locale::Locale;
use crate::plural::{plural_category, PluralCategory};

/// `intl.pluralize(n: number, forms: object, locale?: string) → string`
///
/// Selects the appropriate plural form for a number based on CLDR plural rules.
///
/// `forms` is an object with keys matching plural category names:
/// `"zero"`, `"one"`, `"two"`, `"few"`, `"many"`, `"other"`.
/// Missing categories fall back to `"other"`.
///
/// Examples:
/// - `intl.pluralize(1, {one: "item", other: "items"})` → `"item"`
/// - `intl.pluralize(5, {one: "item", other: "items"})` → `"items"`
/// - `intl.pluralize(0, {zero: "no items", one: "item", other: "items"})` → `"no items"`
/// - `intl.pluralize(2, {one: "item", two: "items", few: "items", many: "items", other: "items"}, "ar")` → `"items"`
#[derive(Clone, Copy)]
pub struct Pluralize;

impl Function for Pluralize {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let forms = require_object(args, 1)?;
        let locale = optional_locale(args, 2)?;

        let category = plural_category(&locale, n);
        let key = category.as_str();

        let form = forms
            .get(key)
            .or_else(|| forms.get("other"))
            .ok_or_else(|| Error::PrimitiveError(format!(
                "pluralize: missing 'other' fallback in forms object"
            )))?;

        let form_str = form.as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;

        Ok(s(form_str))
    }
}
