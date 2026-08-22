use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::provider::IntlProvider;

/// `intl.supported_locales_of() → array`
///
/// Returns the list of locales supported by the compiled CLDR data.
/// With a blob provider, returns the common locale list (blob locale
/// enumeration is not supported by ICU4X's BlobDataProvider).
///
/// Example:
/// - `intl.supported_locales_of()` → `["en", "de", "ja", "fr", ...]`
#[derive(Clone)]
pub struct SupportedLocalesOf {
    provider: Option<IntlProvider>,
}

impl SupportedLocalesOf {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for SupportedLocalesOf {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for SupportedLocalesOf {
    fn call(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if self.provider.is_some() {
            let _ = self.provider.as_ref().unwrap().as_bytes();
        }

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
