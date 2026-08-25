use super::test_helpers::*;
use handl_vm_values_json::NonFunctionValue;

// ── rand.seeded_int ──────────────────────────────────────────────

#[test]
fn seeded_int_deterministic() {
    let a = call("seeded_int", &[n(42.0), n(1.0), n(100.0)]);
    let b = call("seeded_int", &[n(42.0), n(1.0), n(100.0)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn seeded_int_in_range() {
    for seed in [0.0, 1.0, 42.0, 999.0, 1e10] {
        let result = call("seeded_int", &[n(seed), n(1.0), n(6.0)]);
        let val = extract_num(&result) as i64;
        assert!(
            (1..=6).contains(&val),
            "seed {seed} produced {val}, expected 1..=6"
        );
    }
}

#[test]
fn seeded_int_different_seeds_differ() {
    let mut values = std::collections::HashSet::new();
    for seed in 0..50 {
        let result = call("seeded_int", &[n(seed as f64), n(0.0), n(1000.0)]);
        values.insert(extract_num(&result) as i64);
    }
    assert!(
        values.len() > 1,
        "different seeds should produce different values"
    );
}

#[test]
fn seeded_int_min_greater_than_max_errors() {
    let result = call_err("seeded_int", &[n(42.0), n(10.0), n(1.0)]);
    assert!(result.is_err());
}

#[test]
fn seeded_int_no_args_errors() {
    let result = call_err("seeded_int", &[]);
    assert!(matches!(result, Err(handl_vm::Error::ArgumentCount { .. })));
}

// ── rand.seeded_float ────────────────────────────────────────────

#[test]
fn seeded_float_deterministic() {
    let a = call("seeded_float", &[n(42.0), n(0.0), n(1.0)]);
    let b = call("seeded_float", &[n(42.0), n(0.0), n(1.0)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn seeded_float_in_range() {
    let result = call("seeded_float", &[n(42.0), n(10.0), n(20.0)]);
    let val = extract_num(&result);
    assert!((10.0..20.0).contains(&val), "got {val}, expected [10, 20)");
}

#[test]
fn seeded_float_min_equals_max() {
    let result = call("seeded_float", &[n(42.0), n(5.0), n(5.0)]);
    assert_eq!(extract_num(&result), 5.0);
}

#[test]
fn seeded_float_no_args_errors() {
    let result = call_err("seeded_float", &[]);
    assert!(matches!(result, Err(handl_vm::Error::ArgumentCount { .. })));
}

// ── rand.seeded_bool ─────────────────────────────────────────────

#[test]
fn seeded_bool_deterministic() {
    let a = call("seeded_bool", &[n(42.0)]);
    let b = call("seeded_bool", &[n(42.0)]);
    assert_eq!(a.as_bool(), b.as_bool());
}

#[test]
fn seeded_bool_returns_boolean() {
    let result = call("seeded_bool", &[n(1.0)]);
    assert!(result.as_bool().is_some());
}

// ── rand.seeded_pick ─────────────────────────────────────────────

#[test]
fn seeded_pick_deterministic() {
    let items = arr(&[
        serde_json::json!("a"),
        serde_json::json!("b"),
        serde_json::json!("c"),
    ]);
    let a = call("seeded_pick", &[n(42.0), items.clone()]);
    let b = call("seeded_pick", &[n(42.0), items]);
    assert_eq!(a.as_str(), b.as_str());
}

#[test]
fn seeded_pick_empty_array_errors() {
    let result = call_err("seeded_pick", &[n(42.0), arr(&[])]);
    assert!(result.is_err());
}

// ── rand.seeded_choice ───────────────────────────────────────────

#[test]
fn seeded_choice_deterministic() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
    ]);
    let a = call("seeded_choice", &[n(42.0), items.clone(), n(5.0)]);
    let b = call("seeded_choice", &[n(42.0), items, n(5.0)]);
    assert_eq!(extract_array(&a), extract_array(&b));
}

#[test]
fn seeded_choice_correct_count() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call("seeded_choice", &[n(42.0), items, n(10.0)]);
    assert_eq!(extract_array(&result).len(), 10);
}

// ── rand.seeded_sample ───────────────────────────────────────────

#[test]
fn seeded_sample_deterministic() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    let a = call("seeded_sample", &[n(42.0), items.clone(), n(3.0)]);
    let b = call("seeded_sample", &[n(42.0), items, n(3.0)]);
    assert_eq!(extract_array(&a), extract_array(&b));
}

#[test]
fn seeded_sample_no_duplicates() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    let result = call("seeded_sample", &[n(42.0), items, n(5.0)]);
    let arr = extract_array(&result);
    let mut seen = std::collections::HashSet::new();
    for v in &arr {
        assert!(seen.insert(v), "duplicate found in seeded sample");
    }
}

// ── rand.seeded_shuffle ──────────────────────────────────────────

#[test]
fn seeded_shuffle_deterministic() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    let a = call("seeded_shuffle", &[n(42.0), items.clone()]);
    let b = call("seeded_shuffle", &[n(42.0), items]);
    assert_eq!(extract_array(&a), extract_array(&b));
}

#[test]
fn seeded_shuffle_preserves_elements() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    let result = call("seeded_shuffle", &[n(42.0), items]);
    let shuffled = extract_array(&result);
    let mut original: Vec<i64> = [1, 2, 3, 4, 5].to_vec();
    let mut shuffled_nums: Vec<i64> = shuffled
        .iter()
        .map(|v| v.as_f64().unwrap() as i64)
        .collect();
    original.sort();
    shuffled_nums.sort();
    assert_eq!(original, shuffled_nums);
}

// ── rand.seeded_string ───────────────────────────────────────────

#[test]
fn seeded_string_deterministic() {
    let a = call("seeded_string", &[n(42.0), n(16.0)]);
    let b = call("seeded_string", &[n(42.0), n(16.0)]);
    assert_eq!(extract_str(&a), extract_str(&b));
}

#[test]
fn seeded_string_correct_length() {
    let result = call("seeded_string", &[n(42.0), n(32.0)]);
    assert_eq!(extract_str(&result).len(), 32);
}

#[test]
fn seeded_string_hex_charset() {
    let result = call(
        "seeded_string",
        &[n(42.0), n(50.0), handl_vm::NonFunctionValue::new("hex")],
    );
    let s = extract_str(&result);
    assert_eq!(s.len(), 50);
    for c in s.chars() {
        assert!(
            c.is_ascii_digit() || ('a'..='f').contains(&c),
            "non-hex character: {c}"
        );
    }
}

#[test]
fn seeded_string_negative_length_errors() {
    let result = call_err("seeded_string", &[n(42.0), n(-1.0)]);
    assert!(result.is_err());
}

// ── rand.seeded_gauss ────────────────────────────────────────────

#[test]
fn seeded_gauss_deterministic() {
    let a = call("seeded_gauss", &[n(42.0), n(0.0), n(1.0)]);
    let b = call("seeded_gauss", &[n(42.0), n(0.0), n(1.0)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn seeded_gauss_returns_finite() {
    let result = call("seeded_gauss", &[n(42.0), n(0.0), n(1.0)]);
    assert!(extract_num(&result).is_finite());
}

#[test]
fn seeded_gauss_zero_stddev_returns_mean() {
    let result = call("seeded_gauss", &[n(42.0), n(100.0), n(0.0)]);
    assert_eq!(extract_num(&result), 100.0);
}

#[test]
fn seeded_gauss_negative_stddev_errors() {
    let result = call_err("seeded_gauss", &[n(42.0), n(0.0), n(-1.0)]);
    assert!(result.is_err());
}

// ── negative seed ────────────────────────────────────────────────

#[test]
fn seeded_negative_seed_works() {
    let a = call("seeded_int", &[n(-1.0), n(0.0), n(100.0)]);
    let b = call("seeded_int", &[n(-1.0), n(0.0), n(100.0)]);
    assert_eq!(extract_num(&a), extract_num(&b));
    let val = extract_num(&a) as i64;
    assert!((0..=100).contains(&val));
}
