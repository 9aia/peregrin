use ecma402_traits::number_format::{NumberFormatPart, NumberFormatPartType};
use ecma402_traits::relative_time::RelativeTimeFormatPart;

/// Splits a relative-time formatted string into `RelativeTimeFormatPart`s by
/// locating the number parts within it.
///
/// `number_parts` should be produced by formatting the numeric `value` with a
/// `NumberFormat` that matches the number rendering used by the relative-time
/// formatter. Any text before, between, or after the number parts becomes
/// `literal` parts.
pub fn split_parts(
    formatted: &str,
    number_parts: &[NumberFormatPart],
) -> Vec<RelativeTimeFormatPart> {
    let mut result = Vec::new();
    let mut remaining = formatted;

    for np in number_parts {
        match remaining.find(&np.value) {
            Some(pos) => {
                let literal = &remaining[..pos];
                if !literal.is_empty() {
                    result.push(RelativeTimeFormatPart {
                        part_type: NumberFormatPartType::Literal,
                        value: literal.to_string(),
                    });
                }
                result.push(RelativeTimeFormatPart {
                    part_type: np.part_type.clone(),
                    value: np.value.clone(),
                });
                remaining = &remaining[pos + np.value.len()..];
            }
            None => {
                // The relative-time pattern may drop some number parts (e.g.
                // a minus sign replaced by "ago"), so continue searching for
                // later parts.
            }
        }
    }

    if !remaining.is_empty() {
        result.push(RelativeTimeFormatPart {
            part_type: NumberFormatPartType::Literal,
            value: remaining.to_string(),
        });
    }

    result
}