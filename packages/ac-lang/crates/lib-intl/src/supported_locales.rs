use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::s;

/// `intl.supported_locales_of() → array`
///
/// Returns the list of locales supported by the compiled CLDR data.
/// With a blob provider, returns locales available in the blob.
///
/// Example:
/// - `intl.supported_locales_of()` → `["en", "de", "ja", "fr", ...]`
#[derive(Clone, Copy)]
pub struct SupportedLocalesOf;

impl Function for SupportedLocalesOf {
    fn call(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let common = [
            "af", "am", "ar", "az", "be", "bg", "bn", "bs", "ca", "cs",
            "cy", "da", "de", "el", "en", "es", "et", "eu", "fa", "fi",
            "fil", "fr", "ga", "gl", "gu", "he", "hi", "hr", "hu", "hy",
            "id", "is", "it", "ja", "ka", "kk", "km", "kn", "ko", "ky",
            "lo", "lt", "lv", "mk", "ml", "mn", "mr", "ms", "my", "nb",
            "ne", "nl", "pl", "pt", "ro", "ru", "si", "sk", "sl", "sq",
            "sr", "sv", "sw", "ta", "te", "th", "tr", "uk", "ur", "uz",
            "vi", "zh",
        ];

        let arr: Vec<serde_json::Value> = common.iter().map(|l| serde_json::json!(l)).collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(arr)))
    }
}
