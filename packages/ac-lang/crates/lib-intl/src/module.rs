use ac_runtime_core::Module;
use ac_vm::Function;

use super::format_list::FormatList;
use super::format_number::FormatNumber;
use super::select_plural::SelectPlural;

/// The `intl` module — internationalization functions.
///
/// Provides functions callable as `intl.format_number(n, locale?, opts?)`,
/// `intl.select_plural(n, locale?, opts?)`,
/// `intl.format_list(items, locale?, opts?)` in AC expressions.
///
/// All functions accept an optional locale argument (BCP 47 tag).
/// If omitted, defaults to the system default locale.
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
            _ => None,
        }
    }
}
