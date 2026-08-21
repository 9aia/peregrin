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
//!   `math.pi`, `math.e`
//! - [`StringModule`] — `string.upper`, `string.lower`, `string.trim`,
//!   `string.trim_start`, `string.trim_end`, `string.split`, `string.join`,
//!   `string.replace`, `string.replace_first`, `string.contains`,
//!   `string.starts_with`, `string.ends_with`, `string.slice`, `string.reverse`,
//!   `string.repeat`, `string.find`, `string.count`, `string.char_at`,
//!   `string.pad_left`, `string.pad_right`, `string.length`, `string.concat`
//! - [`ArrayModule`] — `array.length`, `array.reverse`, `array.concat`,
//!   `array.contains`, `array.index_of`, `array.slice`, `array.first`,
//!   `array.last`, `array.sort`, `array.flatten`, `array.deep_flatten`,
//!   `array.unique`, `array.sum`, `array.avg`, `array.min`, `array.max`,
//!   `array.at`, `array.push`, `array.fill`, `array.difference`,
//!   `array.intersection`, `array.union`
//! - [`BoolModule`] — `bool.and`, `bool.or`, `bool.not`, `bool.xor`,
//!   `bool.nand`, `bool.nor`, `bool.if`, `bool.eq`, `bool.from`
//! - [`FpModule`] — `fp.identity`, `fp.default`, `fp.coalesce`, `fp.is_null`,
//!   `fp.is_bool`, `fp.is_number`, `fp.is_string`, `fp.is_array`, `fp.is_object`,
//!   `fp.type_of`, `fp.is_empty`, `fp.is_truthy`, `fp.range`, `fp.zip`, `fp.chunk`,
//!   `fp.keys`, `fp.values`, `fp.entries`, `fp.pick`, `fp.omit`, `fp.merge`,
//!   `fp.deep_merge`, `fp.map`, `fp.filter`, `fp.reduce`, `fp.flat_map`, `fp.find`,
//!   `fp.some`, `fp.every`, `fp.group_by`, `fp.partition`, `fp.sort_by`,
//!   `fp.sort_by_desc`, `fp.pipe`, `fp.flow`, `fp.compose`, `fp.fanout`,
//!   `fp.choice`, `fp.apply`, `fp.tap`, `fp.compact`, `fp.pluck`, `fp.min_by`,
//!   `fp.max_by`, `fp.count_by`, `fp.deep_get`
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_utils::{MathModule, StringModule, FpModule};
//!
//! let host = ModuleHost::new()
//!     .with(MathModule)
//!     .with(StringModule)
//!     .with(ArrayModule)
//!     .with(BoolModule)
//!     .with(FpModule);
//! ```
//!
//! This crate is interface-agnostic — usable from CLI, MCP, or any consumer
//! that evaluates AC expressions.

mod array;
mod bool;
mod fp;
mod math;
mod string;

pub use array::ArrayModule;
pub use bool::BoolModule;
pub use fp::FpModule;
pub use math::MathModule;
pub use string::StringModule;
