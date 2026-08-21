use ac_runtime_core::Module;
use ac_vm::Function;

use super::format_list::FormatList;
use super::format_number::FormatNumber;
use super::humanize::HumanizeDuration;
use super::ordinal::Ordinal;
use super::pluralize::Pluralize;
use super::relative_time::RelativeTime;

/// The `intl` module — internationalization functions.
///
/// Provides functions callable as `intl.humanize_duration(s)`,
/// `intl.relative_time(s)`, `intl.pluralize(n, forms)`,
/// `intl.ordinal(n)`, `intl.format_number(n)`,
/// `intl.format_list(items)` in AC expressions.
///
/// All functions accept an optional locale argument (BCP 47 tag).
/// If omitted, defaults to `"en"`.
#[derive(Clone, Copy)]
pub struct IntlModule;

impl Module for IntlModule {
    fn name(&self) -> &str {
        "intl"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "humanize_duration" => Some(&HumanizeDuration),
            "relative_time" => Some(&RelativeTime),
            "pluralize" => Some(&Pluralize),
            "ordinal" => Some(&Ordinal),
            "format_number" => Some(&FormatNumber),
            "format_list" => Some(&FormatList),
            _ => None,
        }
    }
}
