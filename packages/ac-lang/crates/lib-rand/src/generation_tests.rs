
use ac_runtime_values::NonFunctionValue;
use super::test_helpers::*;

// ── rand.string ──────────────────────────────────────────────────

#[test]
fn string_correct_length() {
    for len in [1, 8, 16, 32, 64] {
        let result = call("string", &[n(len as f64)]);
        let s = extract_str(&result);
        assert_eq!(s.len(), len as usize, "expected length {len}");
    }
}

#[test]
fn string_zero_length() {
    let result = call("string", &[n(0.0)]);
    assert_eq!(extract_str(&result), "");
}

#[test]
fn string_alphanumeric_only() {
    let result = call("string", &[n(100.0)]);
    let s = extract_str(&result);
    for c in s.chars() {
        assert!(c.is_ascii_alphanumeric(), "non-alphanumeric character: {c}");
    }
}

#[test]
fn string_negative_length_errors() {
    let result = call_err("string", &[n(-1.0)]);
    assert!(result.is_err());
}

#[test]
fn string_hex_charset() {
    let result = call("string", &[n(100.0), NonFunctionValue::new("hex")]);
    let s = extract_str(&result);
    assert_eq!(s.len(), 100);
    for c in s.chars() {
        assert!(
            c.is_ascii_digit() || ('a'..='f').contains(&c),
            "non-hex character: {c}"
        );
    }
}

#[test]
fn string_numeric_charset() {
    let result = call("string", &[n(50.0), NonFunctionValue::new("numeric")]);
    let s = extract_str(&result);
    assert_eq!(s.len(), 50);
    for c in s.chars() {
        assert!(c.is_ascii_digit(), "non-numeric character: {c}");
    }
}

#[test]
fn string_alpha_charset() {
    let result = call("string", &[n(50.0), NonFunctionValue::new("alpha")]);
    let s = extract_str(&result);
    assert_eq!(s.len(), 50);
    for c in s.chars() {
        assert!(c.is_ascii_alphabetic(), "non-alpha character: {c}");
    }
}

#[test]
fn string_lower_charset() {
    let result = call("string", &[n(50.0), NonFunctionValue::new("lower")]);
    let s = extract_str(&result);
    assert_eq!(s.len(), 50);
    for c in s.chars() {
        assert!(c.is_ascii_lowercase(), "non-lower character: {c}");
    }
}

#[test]
fn string_upper_charset() {
    let result = call("string", &[n(50.0), NonFunctionValue::new("upper")]);
    let s = extract_str(&result);
    assert_eq!(s.len(), 50);
    for c in s.chars() {
        assert!(c.is_ascii_uppercase(), "non-upper character: {c}");
    }
}

#[test]
fn string_invalid_charset_errors() {
    let result = call_err("string", &[n(10.0), NonFunctionValue::new("binary")]);
    assert!(result.is_err());
}

#[test]
fn string_non_string_charset_errors() {
    let result = call_err("string", &[n(10.0), n(42.0)]);
    assert!(matches!(
        result,
        Err(ac_runtime::Error::TypeMismatch { .. })
    ));
}

// ── rand.bytes ───────────────────────────────────────────────────

#[test]
fn bytes_correct_length() {
    for len in [1, 8, 16, 32] {
        let result = call("bytes", &[n(len as f64)]);
        let arr = extract_array(&result);
        assert_eq!(arr.len(), len as usize);
    }
}

#[test]
fn bytes_zero_length() {
    let result = call("bytes", &[n(0.0)]);
    assert!(extract_array(&result).is_empty());
}

#[test]
fn bytes_in_range() {
    let result = call("bytes", &[n(100.0)]);
    let arr = extract_array(&result);
    for v in &arr {
        let byte_val = v.as_f64().unwrap();
        assert!(
            (0.0..=255.0).contains(&byte_val),
            "byte {byte_val} out of range"
        );
        assert_eq!(byte_val, byte_val.trunc(), "byte value must be an integer");
    }
}

#[test]
fn bytes_negative_length_errors() {
    let result = call_err("bytes", &[n(-1.0)]);
    assert!(result.is_err());
}

// ── rand.uuid ────────────────────────────────────────────────────

#[test]
fn uuid_format() {
    let result = call("uuid", &[]);
    let uuid = extract_str(&result);
    assert_eq!(uuid.len(), 36, "UUID must be 36 chars including hyphens");
    assert_eq!(uuid.chars().filter(|&c| c == '-').count(), 4);
    let parts: Vec<&str> = uuid.split('-').collect();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0].len(), 8);
    assert_eq!(parts[1].len(), 4);
    assert_eq!(parts[2].len(), 4);
    assert_eq!(parts[3].len(), 4);
    assert_eq!(parts[4].len(), 12);
}

#[test]
fn uuid_version_4() {
    for _ in 0..50 {
        let result = call("uuid", &[]);
        let uuid = extract_str(&result);
        let version_char = uuid.chars().nth(14).unwrap();
        assert_eq!(
            version_char, '4',
            "version nibble must be 4, got {version_char}"
        );
    }
}

#[test]
fn uuid_variant() {
    for _ in 0..50 {
        let result = call("uuid", &[]);
        let uuid = extract_str(&result);
        let variant_char = uuid.chars().nth(19).unwrap();
        assert!(
            matches!(variant_char, '8' | '9' | 'a' | 'b'),
            "variant nibble must be 8/9/a/b, got {variant_char}"
        );
    }
}

#[test]
fn uuid_unique() {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..100 {
        let result = call("uuid", &[]);
        let uuid = extract_str(&result);
        assert!(seen.insert(uuid), "duplicate UUID generated");
    }
}

#[test]
fn uuid_hex_only() {
    let result = call("uuid", &[]);
    let uuid = extract_str(&result);
    for c in uuid.chars().filter(|&c| c != '-') {
        assert!(c.is_ascii_hexdigit(), "non-hex character: {c}");
    }
}