//! # ac-lib-intl
//!
//! Internationalization library for the AC runtime, backed by ICU4X.
//!
//! Provides locale-aware functions for formatting numbers, selecting
//! plural categories, formatting lists, relative time, dates, durations,
//! comparing strings, display names, and text segmentation.
//!
//! ## Architecture
//!
//! ```text
//! ac-runtime-core          ac-lib-intl
//! ─────────────            ────────────
//! Module trait             IntlModule
//! ModuleHost               ICU4X compiled_data (600+ locales)
//!                          IntlError → VmError
//!                          *Function structs
//! ```
//!
//! All locale data is provided by ICU4X's `compiled_data` feature,
//! which bundles CLDR data at compile time. This gives full CLDR
//! coverage (600+ locales) with no runtime data loading.
//!
//! For smaller binaries, use [`IntlModule::with_blob_provider`] to
//! load a subset of CLDR data at runtime from a `.postcard` blob.
//!
//! ## Functions
//!
//! - [`IntlModule`] — `intl.format_number`, `intl.select_plural`,
//!   `intl.format_list`, `intl.format_relative_time`, `intl.format_date`,
//!   `intl.format_duration`, `intl.compare_strings`, `intl.display_name`,
//!   `intl.segment`
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_intl::IntlModule;
//!
//! let host = ModuleHost::new()
//!     .with(IntlModule);
//! ```
//!
//! Expressions can then call `intl.format_number(1234567.89, "pt-BR")`,
//! `intl.select_plural(0, "ar")`, `intl.format_list(["a", "b", "c"], "en")`,
//! `intl.format_relative_time(5, "second")`, `intl.format_date("2024-05-08")`,
//! etc.

mod common;
mod compare_strings;
mod display_name;
mod error;
mod format_date;
mod format_duration;
mod format_list;
mod format_number;
mod format_relative_time;
pub mod module;
pub mod provider;
mod segment;
mod select_plural;
mod supported_locales;

pub use error::IntlError;
pub use module::IntlModule;
pub use provider::IntlProvider;

#[cfg(test)]
mod compare_strings_tests;
#[cfg(test)]
mod display_name_tests;
#[cfg(test)]
mod format_date_tests;
#[cfg(test)]
mod format_duration_tests;
#[cfg(test)]
mod format_list_tests;
#[cfg(test)]
mod format_number_tests;
#[cfg(test)]
mod format_relative_time_tests;
#[cfg(test)]
mod provider_tests;
#[cfg(test)]
mod segment_tests;
#[cfg(test)]
mod select_plural_tests;
#[cfg(test)]
mod supported_locales_tests;
