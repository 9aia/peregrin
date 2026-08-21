//! # ac-lib-intl
//!
//! Internationalization library for the AC runtime.
//!
//! Provides locale-aware functions for humanizing durations, formatting
//! relative time, pluralizing nouns, producing ordinal numbers, formatting
//! numbers with locale separators, and formatting lists.
//!
//! ## Architecture
//!
//! ```text
//! ac-runtime-core          ac-lib-intl
//! ─────────────            ────────────
//! Module trait             IntlModule (Copy, zero-alloc)
//! ModuleHost               Locale (Copy, &'static str)
//!                          PluralRule (CLDR rules, static data)
//!                          IntlError → VmError
//!                          *Function structs
//! ```
//!
//! All locale data is compile-time static — no runtime loading, no heap
//! allocation for locale lookups. The `Locale` type is `Copy` and holds
//! `&'static str` slices into the static data tables.
//!
//! ## Supported locales
//!
//! `en`, `pt`, `es`, `fr`, `de`, `ar`, `ru`, `ja`, `zh`, `it`, `nl`
//!
//! ## Functions
//!
//! - [`IntlModule`] — `intl.humanize_duration`, `intl.relative_time`,
//!   `intl.pluralize`, `intl.ordinal`, `intl.format_number`,
//!   `intl.format_list`
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_intl::IntlModule;
//! use ac_lib_utils::{MathModule, StrModule};
//!
//! let host = ModuleHost::new()
//!     .with(MathModule)
//!     .with(StrModule)
//!     .with(IntlModule);
//! ```
//!
//! Expressions can then call `intl.humanize_duration(125)`,
//! `intl.relative_time(-3600)`, `intl.pluralize(1, {one: "item", other: "items"})`,
//! `intl.ordinal(2)`, `intl.format_number(1234567.89, "pt-BR")`,
//! `intl.format_list(["a", "b", "c"])`, etc.

mod common;
mod data;
mod error;
mod format_list;
mod format_number;
mod humanize;
mod locale;
pub mod module;
mod ordinal;
mod plural;
mod pluralize;
mod relative_time;

pub use error::IntlError;
pub use locale::Locale;
pub use module::IntlModule;
pub use plural::{plural_category, PluralCategory, PluralRule};

#[cfg(test)]
mod format_list_tests;
#[cfg(test)]
mod format_number_tests;
#[cfg(test)]
mod humanize_tests;
#[cfg(test)]
mod locale_tests;
#[cfg(test)]
mod ordinal_tests;
#[cfg(test)]
mod plural_tests;
#[cfg(test)]
mod pluralize_tests;
#[cfg(test)]
mod relative_time_tests;
