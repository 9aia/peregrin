use ac_runtime_core::Module;
use ac_vm::Function;

use super::compare_strings::CompareStrings;
use super::display_name::DisplayName;
use super::format_date::FormatDate;
use super::format_duration::FormatDuration;
use super::format_list::FormatList;
use super::format_number::FormatNumber;
use super::format_relative_time::FormatRelativeTime;
use super::provider::IntlProvider;
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
/// By default, uses ICU4X compiled CLDR data (600+ locales).
/// For smaller binaries, construct with [`IntlModule::with_provider`]
/// to use a runtime-loaded data blob.
pub struct IntlModule {
    provider: Option<IntlProvider>,
    f_format_number: FormatNumber,
    f_select_plural: SelectPlural,
    f_format_list: FormatList,
    f_format_relative_time: FormatRelativeTime,
    f_format_date: FormatDate,
    f_format_duration: FormatDuration,
    f_compare_strings: CompareStrings,
    f_display_name: DisplayName,
    f_segment: Segment,
}

impl Default for IntlModule {
    fn default() -> Self {
        Self::new()
    }
}

impl IntlModule {
    /// Create a new `IntlModule` using ICU4X compiled CLDR data.
    pub fn new() -> Self {
        Self {
            provider: None,
            f_format_number: FormatNumber,
            f_select_plural: SelectPlural,
            f_format_list: FormatList,
            f_format_relative_time: FormatRelativeTime,
            f_format_date: FormatDate,
            f_format_duration: FormatDuration,
            f_compare_strings: CompareStrings,
            f_display_name: DisplayName,
            f_segment: Segment,
        }
    }

    /// Create a new `IntlModule` using a runtime-loaded data blob.
    ///
    /// The blob must be in ICU4X postcard format (`.postcard`).
    /// Generate one with `icu4x-datagen`:
    /// ```sh
    /// icu4x-datagen --keys all --locales en,de,ja --format blob --out data.postcard
    /// ```
    pub fn with_provider(provider: IntlProvider) -> Self {
        Self {
            provider: Some(provider.clone()),
            f_format_number: FormatNumber,
            f_select_plural: SelectPlural,
            f_format_list: FormatList,
            f_format_relative_time: FormatRelativeTime,
            f_format_date: FormatDate,
            f_format_duration: FormatDuration,
            f_compare_strings: CompareStrings,
            f_display_name: DisplayName,
            f_segment: Segment,
        }
    }

    /// Create a new `IntlModule` from a blob of ICU4X postcard data.
    ///
    /// The blob is not validated until the first function call.
    pub fn with_blob_provider(blob: Vec<u8>) -> Self {
        Self::with_provider(IntlProvider::from_blob(blob))
    }

    /// Returns the data provider, if one was set.
    pub fn provider(&self) -> Option<&IntlProvider> {
        self.provider.as_ref()
    }
}

impl Module for IntlModule {
    fn name(&self) -> &str {
        "intl"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "format_number" => Some(&self.f_format_number),
            "select_plural" => Some(&self.f_select_plural),
            "format_list" => Some(&self.f_format_list),
            "format_relative_time" => Some(&self.f_format_relative_time),
            "format_date" => Some(&self.f_format_date),
            "format_duration" => Some(&self.f_format_duration),
            "compare_strings" => Some(&self.f_compare_strings),
            "display_name" => Some(&self.f_display_name),
            "segment" => Some(&self.f_segment),
            _ => None,
        }
    }
}
