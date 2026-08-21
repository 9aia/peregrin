//! # ac-lib-intl
//!
//! Internationalization library for the AC runtime, backed by ICU4X.
//!
//! Provides locale-aware functions for formatting numbers, selecting
//! plural categories, and formatting lists. Phase 2 will add
//! relative time, date formatting, duration formatting, collation,
//! display names, and segmentation.
//!
//! ## Architecture
//!
//! ```text
//! ac-runtime-core          ac-lib-intl
//! ─────────────            ────────────
//! Module trait             IntlModule (Copy)
//! ModuleHost               ICU4X compiled_data (600+ locales)
//!                          IntlError → VmError
//!                          *Function structs
//! ```
//!
//! All locale data is provided by ICU4X's `compiled_data` feature,
//! which bundles CLDR data at compile time. This gives full CLDR
//! coverage (600+ locales) with no runtime data loading.
//!
//! ## Functions (Phase 1)
//!
//! - [`IntlModule`] — `intl.format_number`, `intl.select_plural`,
//!   `intl.format_list`
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
//! etc.

mod common;
mod error;
mod format_list;
mod format_number;
pub mod module;
mod select_plural;

pub use error::IntlError;
pub use module::IntlModule;

#[cfg(test)]
mod format_list_tests;
#[cfg(test)]
mod format_number_tests;
#[cfg(test)]
mod select_plural_tests;
