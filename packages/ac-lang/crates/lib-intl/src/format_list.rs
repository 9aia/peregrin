use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{optional_locale, require_array, s};

/// `intl.format_list(items: array, locale?: string, style?: string) → string`
///
/// Formats an array of strings into a locale-aware list string.
///
/// `style` can be `"long"` (default) or `"short"`. Currently both produce
/// the same output; the parameter is accepted for forward compatibility.
///
/// Examples:
/// - `intl.format_list(["a", "b", "c"])` → `"a, b, and c"`
/// - `intl.format_list(["a", "b"])` → `"a and b"`
/// - `intl.format_list(["a"])` → `"a"`
/// - `intl.format_list(["a", "b", "c"], "pt")` → `"a, b e c"`
#[derive(Clone, Copy)]
pub struct FormatList;

impl Function for FormatList {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let items = require_array(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let data = locale.data();

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

        let patterns = &data.list_conj;
        let result = format_list_impl(&strings, patterns);

        Ok(s(result))
    }
}

fn format_list_impl(items: &[&str], patterns: &crate::data::ListPatterns) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{}{}{}", items[0], patterns.pair, items[1]),
        _ => {
            let mut result = String::new();
            result.push_str(items[0]);
            for i in 1..items.len() - 1 {
                result.push_str(patterns.middle);
                result.push_str(items[i]);
            }
            result.push_str(patterns.end);
            result.push_str(items[items.len() - 1]);
            result
        }
    }
}
