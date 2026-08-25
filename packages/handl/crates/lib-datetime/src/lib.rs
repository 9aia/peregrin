//! # handl-lib-datetime
//!
//! Date and time utility library for the handl runtime.
//!
//! Provides functions for getting the current time, formatting/parsing
//! ISO 8601 datetime strings, extracting components (year, month, day, etc.),
//! performing arithmetic (add seconds, days, months, etc.), comparing
//! datetimes, and computing durations.
//!
//! ## Architecture
//!
//! ```text
//! handl-vm-common          handl-lib-datetime
//! ─────────────            ────────────────
//! Module trait             DateTimeModule (Copy, zero-alloc)
//! ModuleHost               *Function structs
//!                          chrono → VmError
//! ```
//!
//! All function structs are `Copy` with no internal state.
//!
//! ## Functions
//!
//! - [`DateTimeModule`] — `datetime.from_unix`, `datetime.to_unix`,
//!   `datetime.from_components`, `datetime.format`,
//!   `datetime.parse`, `datetime.to_timezone`, `datetime.to_utc`,
//!   `datetime.year`, `datetime.month`, `datetime.day`,
//!   `datetime.hour`, `datetime.minute`, `datetime.second`,
//!   `datetime.weekday`, `datetime.day_of_year`, `datetime.week_of_year`,
//!   `datetime.quarter`, `datetime.is_leap_year`, `datetime.is_weekend`,
//!   `datetime.add_seconds`, `datetime.add_minutes`, `datetime.add_hours`,
//!   `datetime.add_days`, `datetime.add_weeks`, `datetime.add_months`,
//!   `datetime.add_years`, `datetime.compare`, `datetime.is_before`,
//!   `datetime.is_after`, `datetime.is_equal`, `datetime.is_between`,
//!   `datetime.is_between_inclusive`,
//!   `datetime.duration_between`, `datetime.duration_seconds`,
//!   `datetime.duration_minutes`, `datetime.duration_hours`,
//!   `datetime.duration_days`,
//!   `datetime.start_of_day`, `datetime.end_of_day`,
//!   `datetime.start_of_hour`, `datetime.end_of_hour`,
//!   `datetime.start_of_month`, `datetime.end_of_month`,
//!   `datetime.start_of_year`, `datetime.end_of_year`,
//!   `datetime.is_valid`
//!
//! ## Usage
//!
//! ```ignore
//! use handl_vm_common::ModuleHost;
//! use handl_lib_datetime::DateTimeModule;
//!
//! let host = ModuleHost::new()
//!     .with(DateTimeModule);
//! ```

mod datetime;
mod error;

pub use datetime::DateTimeModule;
pub use error::DatetimeError;
