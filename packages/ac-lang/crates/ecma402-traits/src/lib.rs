//! Platform-agnostic ECMA-402-style trait definitions.
//!
//! This crate contains the interface surface for ECMA-402 APIs. Backends,
//! such as the ICU4X-backed `ecma402-icu4x`, provide the concrete
//! implementations.
//!
//! The modules mirror the ECMA-402 `Intl` constructors:
//! `Collator`, `DateTimeFormat`, `DisplayNames`, `ListFormat`, `Locale`,
//! `NumberFormat`, `PluralRules`, `RelativeTimeFormat`, and `Segmenter`.
//! Additional utilities for locale negotiation live in [`locale_match`].
//!
//! # Implementing a backend
//!
//! A backend crate implements the public traits for a single type or a set of
//! types. The error type is left opaque so that each backend can surface its own
//! failure mode.
//!
//! ```rust
//! use ecma402_traits::plural_rules::{PluralRules, PluralRulesOptions, PluralRuleType, PluralCategory};
//!
//! #[derive(Debug)]
//! struct SimpleRules;
//!
//! impl PluralRules for SimpleRules {
//!     type Error = std::convert::Infallible;
//!
//!     fn select(&self, n: f64) -> Result<PluralCategory, Self::Error> {
//!         Ok(if n == 1.0 { PluralCategory::One } else { PluralCategory::Other })
//!     }
//! }
//!
//! let rules = SimpleRules;
//! assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
//! assert_eq!(rules.select(5.0).unwrap(), PluralCategory::Other);
//! ```

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
