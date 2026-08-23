use ecma402_traits::list_format::{ListFormatPart, ListFormatPartType};

/// Returns a placeholder string for `index` that is extremely unlikely to
/// collide with real list item text or CLDR pattern literals.
pub fn placeholder_for(index: usize) -> String {
    format!("\u{E000}{}\u{E001}", index)
}

/// Returns the placeholder strings for `count` items.
pub fn placeholders(count: usize) -> Vec<String> {
    (0..count).map(placeholder_for).collect()
}

/// Splits a list formatted from `placeholders` back into spec-aligned
/// `element` / `literal` parts using the original item values.
///
/// Callers format the list once with `placeholders` to obtain `formatted`,
/// then pass the original `items` here so the returned parts contain the
/// real values while the separators are taken from the formatted pattern.
pub fn split_parts(items: &[&str], formatted: &str) -> Vec<ListFormatPart> {
    let mut result = Vec::new();
    if items.is_empty() {
        return result;
    }

    let ph = placeholders(items.len());
    let mut remaining = formatted;

    for (i, item) in items.iter().enumerate() {
        let marker = &ph[i];
        match remaining.find(marker) {
            Some(pos) => {
                let literal = &remaining[..pos];
                if !literal.is_empty() {
                    result.push(ListFormatPart {
                        part_type: ListFormatPartType::Literal,
                        value: literal.to_string(),
                    });
                }
                result.push(ListFormatPart {
                    part_type: ListFormatPartType::Element,
                    value: (*item).to_string(),
                });
                remaining = &remaining[pos + marker.len()..];
            }
            None => {
                // Pattern doesn't contain the expected marker; dump the rest.
                if !remaining.is_empty() {
                    result.push(ListFormatPart {
                        part_type: ListFormatPartType::Literal,
                        value: remaining.to_string(),
                    });
                }
                return result;
            }
        }
    }

    if !remaining.is_empty() {
        result.push(ListFormatPart {
            part_type: ListFormatPartType::Literal,
            value: remaining.to_string(),
        });
    }

    result
}
