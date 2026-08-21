use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::locale::names::{
    LanguageIdentifierDisplayName, RegionDisplayName, ScriptDisplayName,
    VariantDisplayName,
};
use writeable::Writeable;

use super::common::{opt_str, optional_locale, optional_object, require_string, s};
use super::error::IntlError;

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
#[derive(Clone, Copy)]
pub struct DisplayName;

impl Function for DisplayName {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let code = require_string(args, 0)?;
        let name_type = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let _opts = optional_object(args, 3)?;

        let formatted = match name_type {
            "language" => {
                let display = LanguageIdentifierDisplayName::try_new_light(locale.into())
                    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                let lang_id = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid language code: {e}") }.into_vm_error())?;
                display.of(&lang_id).write_to_string().into_owned()
            }
            "region" => {
                let display = RegionDisplayName::new_light_with_fallback(locale.into());
                let region = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid region code: {e}") }.into_vm_error())?;
                display.of(&region).write_to_string().into_owned()
            }
            "script" => {
                let display = ScriptDisplayName::new_light_with_fallback(locale.into());
                let script = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid script code: {e}") }.into_vm_error())?;
                display.of(&script).write_to_string().into_owned()
            }
            "variant" => {
                let display = VariantDisplayName::new_heavy_with_fallback(locale.into());
                let variant = code.parse()
                    .map_err(|e| IntlError::InvalidArgument { message: format!("invalid variant code: {e}") }.into_vm_error())?;
                display.of(&variant).write_to_string().into_owned()
            }
            _ => return Err(IntlError::InvalidType {
                type_name: name_type.to_string(),
                valid: "language, region, script, variant".into(),
            }.into_vm_error()),
        };

        Ok(s(formatted))
    }
}
