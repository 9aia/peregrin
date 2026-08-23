//! Type compatibility checking: Rust reference, AC-Lang function, and runner.

mod eval;
mod function;
mod reference;

pub use eval::eval_is_compatible_ac_lang;
pub(crate) use function::IS_COMPATIBLE_BODY;
pub use function::is_compatible_function_value;
pub use reference::is_compatible_json;
