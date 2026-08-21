//! # ac-lib-utils
//!
//! Standard library modules for the AC runtime.
//!
//! Provides concrete [`Module`] implementations with useful functions for
//! expressions and interpolation:
//!
//! - [`MathModule`] — `math.abs`, `math.floor`, `math.ceil`, `math.round`,
//!   `math.min`, `math.max`, `math.sqrt`, `math.pow`, `math.sign`, `math.trunc`,
//!   `math.clamp`, `math.log`, `math.log10`, `math.log2`, `math.exp`, `math.sin`,
//!   `math.cos`, `math.tan`, `math.asin`, `math.acos`, `math.atan`, `math.atan2`,
//!   `math.hypot`, `math.mod`, `math.cbrt`, `math.degrees`, `math.radians`,
//!   `math.round_to`, `math.pi`, `math.e`
//! - [`StrModule`] — `str.upper`, `str.lower`, `str.trim`,
//!   `str.trim_start`, `str.trim_end`, `str.split`, `str.join`,
//!   `str.replace`, `str.replace_first`, `str.contains`,
//!   `str.starts_with`, `str.ends_with`, `str.slice`, `str.reverse`,
//!   `str.repeat`, `str.find`, `str.count`, `str.char_at`,
//!   `str.pad_left`, `str.pad_right`, `str.length`, `str.concat`,
//!   `str.strip_prefix`, `str.strip_suffix`, `str.lines`, `str.rfind`,
//!   `str.capitalize`, `str.title_case`, `str.snake_case`, `str.camel_case`,
//!   `str.pascal_case`, `str.kebab_case`, `str.split_at`, `str.trim_matches`,
//!   `str.is_empty`, `str.equals`
//! - [`ArrayModule`] — `array.length`, `array.reverse`, `array.concat`,
//!   `array.contains`, `array.index_of`, `array.slice`, `array.first`,
//!   `array.last`, `array.sort`, `array.flatten`, `array.deep_flatten`,
//!   `array.unique`, `array.sum`, `array.avg`, `array.min`, `array.max`,
//!   `array.at`, `array.push`, `array.pop`, `array.shift`, `array.fill`,
//!   `array.difference`, `array.intersection`, `array.union`,
//!   `array.take`, `array.drop`, `array.take_while`, `array.drop_while`,
//!   `array.enumerate`, `array.unzip`, `array.scan`, `array.foldr`,
//!   `array.intersperse`, `array.intercalate`, `array.unique_by`,
//!   `array.symmetric_difference`, `array.find_index`, `array.find_last`,
//!   `array.find_last_index`, `array.dedupe`, `array.prepend`, `array.without`,
//!   `array.range`, `array.zip`, `array.chunk`, `array.map`, `array.filter`,
//!   `array.reduce`, `array.flat_map`, `array.find`, `array.some`, `array.every`,
//!   `array.group_by`, `array.partition`, `array.sort_by`, `array.sort_by_desc`,
//!   `array.compact`, `array.pluck`, `array.min_by`, `array.max_by`,
//!   `array.count_by`
//! - [`BoolModule`] — `bool.and`, `bool.or`, `bool.not`, `bool.xor`,
//!   `bool.nand`, `bool.nor`, `bool.implies`, `bool.eq`
//! - [`ObjectModule`] — `object.keys`, `object.values`, `object.entries`,
//!   `object.pick`, `object.omit`, `object.merge`, `object.deep_get`,
//!   `object.deep_merge`
//! - [`FnModule`] — `fn.pipe`, `fn.flow`, `fn.compose`, `fn.fanout`,
//!   `fn.choice`, `fn.apply`, `fn.tap`
//! - [`TypeModule`] — `type.to_string`, `type.to_number`, `type.to_bool`,
//!   `type.is_null`, `type.is_bool`, `type.is_number`, `type.is_string`,
//!   `type.is_array`, `type.is_object`, `type.type_of`, `type.is_empty`,
//!   `type.is_truthy`, `type.identity`, `type.default`, `type.coalesce`
//! - [`DateTimeModule`] — `datetime.now`, `datetime.utc_now`,
//!   `datetime.from_unix`, `datetime.to_unix`, `datetime.format`,
//!   `datetime.parse`, `datetime.year`, `datetime.month`, `datetime.day`,
//!   `datetime.hour`, `datetime.minute`, `datetime.second`,
//!   `datetime.weekday`, `datetime.day_of_year`, `datetime.week_of_year`,
//!   `datetime.quarter`, `datetime.is_leap_year`, `datetime.is_weekend`,
//!   `datetime.add_seconds`, `datetime.add_minutes`, `datetime.add_hours`,
//!   `datetime.add_days`, `datetime.add_weeks`, `datetime.add_months`,
//!   `datetime.add_years`, `datetime.compare`, `datetime.is_before`,
//!   `datetime.is_after`, `datetime.is_equal`, `datetime.is_between`,
//!   `datetime.duration_between`, `datetime.duration_seconds`,
//!   `datetime.humanize_duration`, `datetime.is_valid`
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_utils::{MathModule, StrModule, FpModule};
//!
//! let host = ModuleHost::new()
//!     .with(MathModule)
//!     .with(StrModule)
//!     .with(ArrayModule)
//!     .with(BoolModule)
//!     .with(FpModule)
//!     .with(TypeModule)
//!     .with(DateTimeModule);
//! ```
//!
//! This crate is interface-agnostic — usable from CLI, MCP, or any consumer
//! that evaluates AC expressions.

mod array;
mod bool;
mod datetime;
mod fp;
mod math;
mod r#str;
mod r#type;

pub use array::ArrayModule;
pub use bool::BoolModule;
pub use datetime::DateTimeModule;
pub use fp::FpModule;
pub use math::MathModule;
pub use r#str::StrModule;
pub use r#type::TypeModule;
