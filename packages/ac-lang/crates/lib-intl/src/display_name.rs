use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::locale::names::{
    LanguageIdentifierDisplayName, LanguageIdentifierDisplayNameOptions,
    RegionDisplayName, ScriptDisplayName,
    VariantDisplayName,
};
use writeable::Writeable;

use super::common::{make_provider, optional_locale, optional_object, require_string, s};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.display_name(code: string, type: string, locale?: string, opts?: object) → string`
///
/// Returns the display name for a language, region, script, or variant code.
///
/// `type` must be one of: `"language"`, `"region"`, `"script"`, or `"variant"`.
///
/// Examples:
/// - `intl.display_name("en", "language", "pt")` → `"Inglês"`
/// - `intl.display_name("US", "region", "en")` → `"United States"`
/// - `intl.display_name("Latn", "script", "en")` → `"Latin"`
/// - `intl.display_name("fonipa", "variant", "en")` → `"IPA Phonetics"`
#[derive(Clone)]
pub struct DisplayName {
    provider: Option<IntlProvider>,
}

impl DisplayName {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for DisplayName {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for DisplayName {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let code = require_string(args, 0)?;
        let name_type = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let _opts = optional_object(args, 3)?;
        let dp = make_provider(&self.provider)?;

        let formatted = match name_type {
            "language" => {
                let lang_id = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid language code: {e}") }.into_vm_error())?;
                let display = match dp.as_buffer_provider() {
                    Some(p) => LanguageIdentifierDisplayName::try_new_light_with_buffer_provider(
                        p, locale.into(), lang_id, LanguageIdentifierDisplayNameOptions::default(),
                    ),
                    None => LanguageIdentifierDisplayName::try_new_light(
                        locale.into(), lang_id, LanguageIdentifierDisplayNameOptions::default(),
                    ),
                }
                .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                display.as_borrowed().write_to_string().into_owned()
            }
            "region" => {
                let region = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid region code: {e}") }.into_vm_error())?;
                let display = match dp.as_buffer_provider() {
                    Some(p) => RegionDisplayName::try_new_light_with_buffer_provider(p, locale.into(), region)
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?,
                    None => RegionDisplayName::new_light_with_fallback(locale.into(), region),
                };
                display.write_to_string().into_owned()
            }
            "script" => {
                let script = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid script code: {e}") }.into_vm_error())?;
                let display = match dp.as_buffer_provider() {
                    Some(p) => ScriptDisplayName::try_new_light_with_buffer_provider(p, locale.into(), script)
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?,
                    None => ScriptDisplayName::new_light_with_fallback(locale.into(), script),
                };
                display.write_to_string().into_owned()
            }
            "variant" => {
                let variant = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid variant code: {e}") }.into_vm_error())?;
                let display = match dp.as_buffer_provider() {
                    Some(p) => VariantDisplayName::try_new_heavy_with_buffer_provider(p, locale.into(), variant)
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?,
                    None => VariantDisplayName::new_heavy_with_fallback(locale.into(), variant),
                };
                display.write_to_string().into_owned()
            }
            _ => return Err(IntlError::InvalidType {
                type_name: name_type.to_string(),
                valid: "language, region, script, variant".into(),
            }.into_vm_error()),
        };

        Ok(s(formatted))
    }
}
