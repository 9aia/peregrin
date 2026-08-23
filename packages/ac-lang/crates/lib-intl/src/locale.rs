use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::locale::Locale;
use icu::locale::extensions::unicode::{Key, Value};
use icu::locale::subtags::{Language, Script, Region};

use super::common::{optional_object, require_string};
use super::error::IntlError;

/// `intl.locale(tag: string, opts?: object) → object`
///
/// Parses a BCP 47 locale tag and returns an object with its components.
///
/// This is equivalent to `Intl.Locale` in the ECMA-402 spec.
///
/// Options:
/// - `language`: override the language subtag
/// - `script`: override the script subtag
/// - `region`: override the region subtag
/// - `calendar`: add a `-u-ca-` extension
/// - `numberingSystem`: add a `-u-nu-` extension
/// - `hourCycle`: add a `-u-hc-` extension (`"h11"`, `"h12"`, `"h23"`, `"h24"`)
///
/// Returns an object with:
/// - `baseName`: the canonical locale identifier (e.g. `"en-US"`)
/// - `language`: the language subtag (e.g. `"en"`)
/// - `script`: the script subtag (e.g. `"Latn"`), or `null`
/// - `region`: the region subtag (e.g. `"US"`), or `null`
/// - `calendar`: the calendar extension, or `null`
/// - `numberingSystem`: the numbering system extension, or `null`
/// - `hourCycle`: the hour cycle extension, or `null`
///
/// Examples:
/// - `intl.locale("en-US")` → `{baseName: "en-US", language: "en", script: null, region: "US", ...}`
/// - `intl.locale("zh-Hant-TW")` → `{baseName: "zh-Hant-TW", language: "zh", script: "Hant", region: "TW", ...}`
/// - `intl.locale("en", {region: "GB"})` → `{baseName: "en-GB", language: "en", script: null, region: "GB", ...}`
#[derive(Clone)]
pub struct LocaleInfo;

impl Default for LocaleInfo {
    fn default() -> Self {
        Self
    }
}

impl Function for LocaleInfo {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let tag = require_string(args, 0)?;
        let opts = optional_object(args, 1)?;

        let mut locale: Locale = tag.parse()
            .map_err(|e| IntlError::InvalidLocale { input: format!("{tag}: {e}") }.into_vm_error())?;

        if let Some(o) = opts {
            if let Some(lang) = o.get("language").and_then(|v| v.as_str()) {
                let parsed: Language = lang.parse()
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid language: {e}") }.into_vm_error())?;
                locale.id.language = parsed;
            }
            if let Some(script) = o.get("script").and_then(|v| v.as_str()) {
                let parsed: Script = script.parse()
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid script: {e}") }.into_vm_error())?;
                locale.id.script = Some(parsed);
            }
            if let Some(region) = o.get("region").and_then(|v| v.as_str()) {
                let parsed: Region = region.parse()
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid region: {e}") }.into_vm_error())?;
                locale.id.region = Some(parsed);
            }
            if let Some(cal) = o.get("calendar").and_then(|v| v.as_str()) {
                let key: Key = "ca".parse()
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid calendar key: {e}") }.into_vm_error())?;
                let val = Value::try_from_str(cal)
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid calendar: {e}") }.into_vm_error())?;
                locale.extensions.unicode.keywords.set(key, val);
            }
            if let Some(nu) = o.get("numberingSystem").and_then(|v| v.as_str()) {
                let key: Key = "nu".parse()
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid numberingSystem key: {e}") }.into_vm_error())?;
                let val = Value::try_from_str(nu)
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid numberingSystem: {e}") }.into_vm_error())?;
                locale.extensions.unicode.keywords.set(key, val);
            }
            if let Some(hc) = o.get("hourCycle").and_then(|v| v.as_str()) {
                let key: Key = "hc".parse()
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid hourCycle key: {e}") }.into_vm_error())?;
                let val = Value::try_from_str(hc)
                    .map_err(|e| IntlError::InvalidLocale { input: format!("invalid hourCycle: {e}") }.into_vm_error())?;
                locale.extensions.unicode.keywords.set(key, val);
            }
        }

        let base_name = locale.to_string();
        let language = locale.id.language.to_string();
        let script = locale.id.script.map(|s| s.to_string());
        let region = locale.id.region.map(|r| r.to_string());

        let cal_key: Key = "ca".parse().unwrap();
        let nu_key: Key = "nu".parse().unwrap();
        let hc_key: Key = "hc".parse().unwrap();
        let calendar = locale.extensions.unicode.keywords.get(&cal_key)
            .and_then(|v| v.as_subtags_slice().first().map(|s| s.to_string()));
        let numbering_system = locale.extensions.unicode.keywords.get(&nu_key)
            .and_then(|v| v.as_subtags_slice().first().map(|s| s.to_string()));
        let hour_cycle = locale.extensions.unicode.keywords.get(&hc_key)
            .and_then(|v| v.as_subtags_slice().first().map(|s| s.to_string()));

        let mut result = serde_json::Map::new();
        result.insert("baseName".to_string(), serde_json::Value::String(base_name));
        result.insert("language".to_string(), serde_json::Value::String(language));
        result.insert("script".to_string(), script.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
        result.insert("region".to_string(), region.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
        result.insert("calendar".to_string(), calendar.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
        result.insert("numberingSystem".to_string(), numbering_system.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
        result.insert("hourCycle".to_string(), hour_cycle.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));

        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}
