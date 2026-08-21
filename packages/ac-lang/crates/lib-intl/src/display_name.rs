use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::locale::names::{
    DisplayNamesPreferences,
    LanguageIdentifierDisplayName,
    LanguageIdentifierDisplayNameOptions,
    RegionDisplayName,
    ScriptDisplayName,
};
use writeable::Writeable;

use super::common::{optional_locale, optional_object, require_string, s};

/// `intl.display_name(code: string, type: string, locale?: string, opts?: object) → string`
///
/// Returns the display name for a language, region, or script code using ICU4X.
///
/// `type` must be one of: `"language"`, `"region"`, `"script"`.
///
/// Examples:
/// - `intl.display_name("en", "language")` → `"English"`
/// - `intl.display_name("US", "region")` → `"United States"`
/// - `intl.display_name("Latn", "script")` → `"Latin"`
/// - `intl.display_name("de", "language", "pt")` → `"alemão"`
#[derive(Clone, Copy)]
pub struct DisplayName;

impl Function for DisplayName {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let code = require_string(args, 0)?;
        let name_type = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let _opts = optional_object(args, 3)?;

        let prefs = DisplayNamesPreferences::from(locale);

        let result = match name_type {
            "language" => {
                let display = LanguageIdentifierDisplayName::try_new_light(
                    prefs,
                    code.parse().map_err(|_| Error::PrimitiveError(format!("invalid language code: {code}")))?,
                    LanguageIdentifierDisplayNameOptions::default(),
                ).map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;
                display.as_borrowed().write_to_string().into_owned()
            }
            "region" => {
                let display = RegionDisplayName::new_light_with_fallback(
                    prefs,
                    code.parse().map_err(|_| Error::PrimitiveError(format!("invalid region code: {code}")))?,
                );
                display.as_borrowed().write_to_string().into_owned()
            }
            "script" => {
                let display = ScriptDisplayName::new_light_with_fallback(
                    prefs,
                    code.parse().map_err(|_| Error::PrimitiveError(format!("invalid script code: {code}")))?,
                );
                display.as_borrowed().write_to_string().into_owned()
            }
            _ => return Err(Error::PrimitiveError(format!("invalid type: {name_type}"))),
        };

        Ok(s(result))
    }
}
