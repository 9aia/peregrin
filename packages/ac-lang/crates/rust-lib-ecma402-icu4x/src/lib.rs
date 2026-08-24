//! ECMA-402 backend implementation using ICU4X.

pub mod collator;
pub mod date_time_format;
pub mod display_names;
pub mod list_format;
pub mod locale;
pub mod locale_match;
pub mod number_format;
pub mod plural_rules;
pub mod relative_time;
pub mod segmenter;

pub use list_format::{Error as ListFormatError, ListFormat};
pub use plural_rules::{Error as PluralRulesError, PluralRules};

#[cfg(test)]
mod collator_tests;
#[cfg(test)]
mod list_format_tests;
#[cfg(test)]
mod plural_rules_tests;
#[cfg(test)]
mod relative_time_tests;