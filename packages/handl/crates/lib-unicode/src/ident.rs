//! Identifier validation for the handl language.
//!
//! Uses the Unicode `XID_Start` and `XID_Continue` properties defined in
//! [UAX #31](https://unicode.org/reports/tr31/) to decide whether a scalar
//! value may appear in a language identifier.

/// Returns `true` if `ch` is a valid first character of an identifier.
pub fn is_identifier_start(ch: char) -> bool {
    ch == '_' || unicode_ident::is_xid_start(ch)
}

/// Returns `true` if `ch` is a valid non-first character of an identifier.
pub fn is_identifier_continue(ch: char) -> bool {
    unicode_ident::is_xid_continue(ch)
}

/// Returns `true` if `value` is a well-formed identifier.
pub fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    if let Some(first) = chars.next() {
        if !is_identifier_start(first) {
            return false;
        }
    } else {
        return false;
    }
    chars.all(is_identifier_continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_identifiers() {
        assert!(is_identifier("foo"));
        assert!(is_identifier("bar123"));
        assert!(is_identifier("_private"));
        assert!(!is_identifier("123abc"));
        assert!(!is_identifier(""));
    }

    #[test]
    fn unicode_identifiers() {
        assert!(is_identifier("αβγ"));
        assert!(is_identifier("Résumé"));
        assert!(!is_identifier("123 é"));
    }
}
