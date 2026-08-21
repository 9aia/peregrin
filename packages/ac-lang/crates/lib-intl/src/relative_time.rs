use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{optional_locale, require_number, s};
use crate::locale::Locale;
use crate::plural::{plural_category, PluralCategory};

/// `intl.relative_time(seconds: number, locale?: string) → string`
///
/// Converts a delta in seconds to a human-readable relative time string
/// using locale-aware templates.
///
/// Examples:
/// - `intl.relative_time(-3600)` → `"1 hour ago"`
/// - `intl.relative_time(3600)` → `"in 1 hour"`
/// - `intl.relative_time(0)` → `"now"`
/// - `intl.relative_time(-3600, "pt")` → `"há 1 hora"`
#[derive(Clone, Copy)]
pub struct RelativeTime;

impl Function for RelativeTime {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let delta_secs = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let data = locale.data();

        let abs_secs = delta_secs.abs();

        if abs_secs < 1.0 {
            return Ok(s(data.now_str));
        }

        let is_past = delta_secs < 0.0;
        let template_set = if is_past { &data.rel_past } else { &data.rel_future };

        let days = (abs_secs / 86400.0).floor() as i64;
        let hours = ((abs_secs % 86400.0) / 3600.0).floor() as i64;
        let minutes = ((abs_secs % 3600.0) / 60.0).floor() as i64;
        let seconds = (abs_secs % 60.0).round() as i64;

        let result = if days > 0 {
            pick_template(days as f64, template_set.day_singular, template_set.day_plural, &locale, days)
        } else if hours > 0 {
            pick_template(hours as f64, template_set.hour_singular, template_set.hour_plural, &locale, hours)
        } else if minutes > 0 {
            pick_template(minutes as f64, template_set.minute_singular, template_set.minute_plural, &locale, minutes)
        } else {
            pick_template(seconds as f64, template_set.second_singular, template_set.second_plural, &locale, seconds)
        };

        Ok(s(result))
    }
}

fn pick_template(
    n: f64,
    singular: &'static str,
    plural: &'static str,
    locale: &Locale,
    count: i64,
) -> String {
    let category = plural_category(locale, n);
    let template = match category {
        PluralCategory::One => singular,
        _ => plural,
    };
    template.replacen("{}", &count.to_string(), 1)
}
