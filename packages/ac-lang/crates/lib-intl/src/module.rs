use ac_runtime_core::Module;
use ac_vm::Function;

use super::compare_strings::CompareStrings;
use super::display_name::DisplayName;
use super::format_date::{FormatDate, FormatDateToParts};
use super::format_duration::FormatDuration;
use super::format_list::FormatList;
use super::format_number::{FormatNumber, FormatNumberToParts};
use super::format_relative_time::FormatRelativeTime;
use super::provider::IntlProvider;
use super::segment::Segment;
use super::select_plural::SelectPlural;
use super::locale_match::{GetCanonicalLocales, SupportedLocalesOf};
use super::locale::LocaleInfo;

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
/// `intl.segment(text, granularity, locale?, opts?)`,
/// `intl.supported_locales_of()` in AC expressions.
///
/// All functions accept an optional locale argument (BCP 47 tag).
/// If omitted, defaults to English.
///
/// By default, uses ICU4X compiled CLDR data (600+ locales).
/// For smaller binaries, construct with [`IntlModule::with_provider`]
/// to use a runtime-loaded data blob.
#[derive(Clone)]
pub struct IntlModule {
    provider: Option<IntlProvider>,
    f_format_number: FormatNumber,
    f_format_number_to_parts: FormatNumberToParts,
    f_select_plural: SelectPlural,
    f_format_list: FormatList,
    f_format_relative_time: FormatRelativeTime,
    f_format_date: FormatDate,
    f_format_date_to_parts: FormatDateToParts,
    f_format_duration: FormatDuration,
    f_compare_strings: CompareStrings,
    f_display_name: DisplayName,
    f_segment: Segment,
    f_supported_locales_of: SupportedLocalesOf,
    f_get_canonical_locales: GetCanonicalLocales,
    f_locale: LocaleInfo,
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
            f_format_number: FormatNumber::new(),
            f_format_number_to_parts: FormatNumberToParts::new(),
            f_select_plural: SelectPlural::new(),
            f_format_list: FormatList::new(),
            f_format_relative_time: FormatRelativeTime::new(),
            f_format_date: FormatDate::new(),
            f_format_date_to_parts: FormatDateToParts::new(),
            f_format_duration: FormatDuration::new(),
            f_compare_strings: CompareStrings::new(),
            f_display_name: DisplayName::new(),
            f_segment: Segment::new(),
            f_supported_locales_of: SupportedLocalesOf::new(),
            f_get_canonical_locales: GetCanonicalLocales,
            f_locale: LocaleInfo,
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
            f_format_number: FormatNumber::with_provider(provider.clone()),
            f_format_number_to_parts: FormatNumberToParts::with_provider(provider.clone()),
            f_select_plural: SelectPlural::with_provider(provider.clone()),
            f_format_list: FormatList::with_provider(provider.clone()),
            f_format_relative_time: FormatRelativeTime::with_provider(provider.clone()),
            f_format_date: FormatDate::with_provider(provider.clone()),
            f_format_date_to_parts: FormatDateToParts::with_provider(provider.clone()),
            f_format_duration: FormatDuration::with_provider(provider.clone()),
            f_compare_strings: CompareStrings::with_provider(provider.clone()),
            f_display_name: DisplayName::with_provider(provider.clone()),
            f_segment: Segment::with_provider(provider.clone()),
            f_supported_locales_of: SupportedLocalesOf::with_provider(provider),
            f_get_canonical_locales: GetCanonicalLocales,
            f_locale: LocaleInfo,
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
            "format_number_to_parts" => Some(&self.f_format_number_to_parts),
            "select_plural" => Some(&self.f_select_plural),
            "format_list" => Some(&self.f_format_list),
            "format_relative_time" => Some(&self.f_format_relative_time),
            "format_date" => Some(&self.f_format_date),
            "format_date_to_parts" => Some(&self.f_format_date_to_parts),
            "format_duration" => Some(&self.f_format_duration),
            "compare_strings" => Some(&self.f_compare_strings),
            "display_name" => Some(&self.f_display_name),
            "segment" => Some(&self.f_segment),
            "supported_locales_of" => Some(&self.f_supported_locales_of),
            "get_canonical_locales" => Some(&self.f_get_canonical_locales),
            "locale" => Some(&self.f_locale),
            _ => None,
        }
    }
}
