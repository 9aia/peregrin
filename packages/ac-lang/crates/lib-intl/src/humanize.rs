use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{optional_locale, require_number, s};
use crate::locale::Locale;
use crate::plural::plural_category;

/// `intl.humanize_duration(seconds: number, locale?: string) → string`
///
/// Converts a duration in seconds to a human-readable string using
/// locale-aware unit names and plural rules.
///
/// Examples:
/// - `intl.humanize_duration(125)` → `"2 minutes 5 seconds"`
/// - `intl.humanize_duration(125, "pt")` → `"2 minutos 5 segundos"`
/// - `intl.humanize_duration(0)` → `"0 seconds"`
#[derive(Clone, Copy)]
pub struct HumanizeDuration;

impl Function for HumanizeDuration {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let total_secs = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let data = locale.data();

        let negative = total_secs < 0.0;
        let abs_secs = total_secs.abs();

        let days = (abs_secs / 86400.0).floor() as i64;
        let hours = ((abs_secs % 86400.0) / 3600.0).floor() as i64;
        let minutes = ((abs_secs % 3600.0) / 60.0).floor() as i64;
        let seconds = (abs_secs % 60.0).round() as i64;

        let mut parts: Vec<String> = Vec::new();

        if days > 0 {
            let unit = pick_unit(days as f64, data.dur_units.day_singular, data.dur_units.day_plural, &locale);
            parts.push(format!("{} {}", days, unit));
        }
        if hours > 0 {
            let unit = pick_unit(hours as f64, data.dur_units.hour_singular, data.dur_units.hour_plural, &locale);
            parts.push(format!("{} {}", hours, unit));
        }
        if minutes > 0 {
            let unit = pick_unit(minutes as f64, data.dur_units.minute_singular, data.dur_units.minute_plural, &locale);
            parts.push(format!("{} {}", minutes, unit));
        }
        if seconds > 0 || parts.is_empty() {
            let unit = pick_unit(seconds as f64, data.dur_units.second_singular, data.dur_units.second_plural, &locale);
            parts.push(format!("{} {}", seconds, unit));
        }

        let mut result = parts.join(" ");
        if negative {
            result = format!("-{result}");
        }

        Ok(s(result))
    }
}

/// Pick the singular or plural unit form based on CLDR plural rules.
fn pick_unit(n: f64, singular: &'static str, plural: &'static str, locale: &Locale) -> &'static str {
    let category = plural_category(locale, n);
    match category {
        crate::plural::PluralCategory::One => singular,
        _ => plural,
    }
}
