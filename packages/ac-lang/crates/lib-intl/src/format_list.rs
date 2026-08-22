use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::list::{ListFormatter, options::{ListFormatterOptions, ListLength}};

use super::common::{opt_str, optional_locale, optional_object, require_array, s};
use super::error::IntlError;

/// `intl.format_list(items: array, locale?: string, opts?: object) → string`
///
/// Formats a list of strings using ICU4X.
///
/// Options:
/// - `type`: `"conjunction"` (default), `"disjunction"`, or `"unit"`
/// - `style`: `"long"` (default), `"short"`, or `"narrow"`
///
/// Examples:
/// - `intl.format_list(["a", "b", "c"])` → `"a, b, and c"`
/// - `intl.format_list(["a", "b", "c"], "en", {type: "disjunction"})` → `"a, b, or c"`
/// - `intl.format_list(["5", "7"], "en", {type: "unit"})` → `"5, 7"`
#[derive(Clone, Copy)]
pub struct FormatList;

impl Function for FormatList {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let items = require_array(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;

        let list_type = opt_str(opts, "type").unwrap_or("conjunction");
        let style = opt_str(opts, "style").unwrap_or("long");

        let length = match style {
            "short" => ListLength::Short,
            "narrow" => ListLength::Narrow,
            _ => ListLength::Wide,
        };
        let options = ListFormatterOptions::default().with_length(length);

        let formatter = match list_type {
            "disjunction" => ListFormatter::try_new_or(locale.into(), options),
            "unit" => ListFormatter::try_new_unit(locale.into(), options),
            _ => ListFormatter::try_new_and(locale.into(), options),
        }
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

        let strings: Vec<&str> = items
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        if strings.is_empty() {
            return Ok(s(""));
        }
        if strings.len() == 1 {
            return Ok(s(strings[0]));
        }

        let formatted = formatter.format_to_string(strings.into_iter());
        Ok(s(formatted))
    }
}
