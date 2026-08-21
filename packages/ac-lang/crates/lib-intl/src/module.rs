use ac_runtime_core::Module;
use ac_vm::Function;

use super::compare_strings::CompareStrings;
use super::display_name::DisplayName;
use super::format_date::FormatDate;
use super::format_duration::FormatDuration;
use super::format_list::FormatList;
use super::format_number::FormatNumber;
use super::format_relative_time::FormatRelativeTime;
use super::segment::Segment;
use super::select_plural::SelectPlural;

/// The `intl` module — internationalization functions.
///
/// Provides functions callable as `intl.format_number(n, locale?, opts?)`,
/// `intl.select_plural(n, locale?, opts?)`,
/// `intl.format_list(items, locale?, opts?)`,
/// `intl.format_relative_time(value, unit, locale?, opts?)`,
/// `intl.format_date(date, locale?, opts?)`,
/// `intl.format_duration(duration, locale?, opts?)`,
/// `intl.compare_strings(a, b, locale?, opts?)`,
/// `intl.display_name(code, type, locale?, opts?)`,
/// `intl.segment(text, granularity, locale?, opts?)` in AC expressions.
///
/// All functions accept an optional locale argument (BCP 47 tag).
/// If omitted, defaults to English.
///
/// Backed by ICU4X with compiled CLDR data (600+ locales).
#[derive(Clone, Copy)]
pub struct IntlModule;

impl Module for IntlModule {
    fn name(&self) -> &str {
        "intl"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "format_number" => Some(&FormatNumber),
            "select_plural" => Some(&SelectPlural),
            "format_list" => Some(&FormatList),
            "format_relative_time" => Some(&FormatRelativeTime),
            "format_date" => Some(&FormatDate),
            "format_duration" => Some(&FormatDuration),
            "compare_strings" => Some(&CompareStrings),
            "display_name" => Some(&DisplayName),
            "segment" => Some(&Segment),
            _ => None,
        }
    }
}
