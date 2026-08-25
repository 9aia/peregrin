use super::test_helpers::*;

// ── rand.pick ────────────────────────────────────────────────────

#[test]
fn pick_returns_element_from_array() {
    let items = arr(&[
        serde_json::json!("a"),
        serde_json::json!("b"),
        serde_json::json!("c"),
    ]);
    for _ in 0..100 {
        let result = call("pick", std::slice::from_ref(&items));
        let val = result.as_str().expect("expected string");
        assert!(val == "a" || val == "b" || val == "c");
    }
}

#[test]
fn pick_empty_array_errors() {
    let result = call_err("pick", &[arr(&[])]);
    assert!(result.is_err());
}

#[test]
fn pick_no_args_errors() {
    let result = call_err("pick", &[]);
    assert!(matches!(result, Err(handl_vm::Error::ArgumentCount { .. })));
}

// ── rand.choice ─────────────────────────────────────────────────

#[test]
fn choice_returns_correct_count() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
    ]);
    let result = call("choice", &[items, n(5.0)]);
    let arr = extract_array(&result);
    assert_eq!(arr.len(), 5);
}

#[test]
fn choice_can_exceed_array_length() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let result = call("choice", &[items, n(10.0)]);
    let arr = extract_array(&result);
    assert_eq!(arr.len(), 10);
    for v in &arr {
        assert!(v == "a" || v == "b", "unexpected value: {v}");
    }
}

#[test]
fn choice_zero_count() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call("choice", &[items, n(0.0)]);
    assert!(extract_array(&result).is_empty());
}

#[test]
fn choice_empty_array_errors() {
    let result = call_err("choice", &[arr(&[]), n(3.0)]);
    assert!(result.is_err());
}

#[test]
fn choice_negative_count_errors() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call_err("choice", &[items, n(-1.0)]);
    assert!(result.is_err());
}

#[test]
fn choice_no_args_errors() {
    let result = call_err("choice", &[]);
    assert!(matches!(result, Err(handl_vm::Error::ArgumentCount { .. })));
}

// ── rand.sample ──────────────────────────────────────────────────

#[test]
fn sample_returns_correct_count() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    let result = call("sample", &[items, n(3.0)]);
    let arr = extract_array(&result);
    assert_eq!(arr.len(), 3);
}

#[test]
fn sample_no_duplicates() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    for _ in 0..50 {
        let result = call("sample", &[items.clone(), n(5.0)]);
        let arr = extract_array(&result);
        assert_eq!(arr.len(), 5);
        let mut seen = std::collections::HashSet::new();
        for v in &arr {
            assert!(seen.insert(v), "duplicate found in sample");
        }
    }
}

#[test]
fn sample_zero_count() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call("sample", &[items, n(0.0)]);
    let arr = extract_array(&result);
    assert!(arr.is_empty());
}

#[test]
fn sample_count_exceeds_length_errors() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call_err("sample", &[items, n(5.0)]);
    assert!(result.is_err());
}

#[test]
fn sample_negative_count_errors() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call_err("sample", &[items, n(-1.0)]);
    assert!(result.is_err());
}

// ── rand.shuffle ─────────────────────────────────────────────────

#[test]
fn shuffle_preserves_elements() {
    let items = arr(&[
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
        serde_json::json!(4),
        serde_json::json!(5),
    ]);
    let result = call("shuffle", std::slice::from_ref(&items));
    let shuffled = extract_array(&result);
    assert_eq!(shuffled.len(), 5);

    let mut original: Vec<i64> = [1, 2, 3, 4, 5].to_vec();
    let mut shuffled_nums: Vec<i64> = shuffled
        .iter()
        .map(|v| v.as_f64().unwrap() as i64)
        .collect();
    original.sort();
    shuffled_nums.sort();
    assert_eq!(original, shuffled_nums);
}

#[test]
fn shuffle_empty_array() {
    let result = call("shuffle", &[arr(&[])]);
    assert!(extract_array(&result).is_empty());
}

#[test]
fn shuffle_single_element() {
    let result = call("shuffle", &[arr(&[serde_json::json!(42)])]);
    let shuffled = extract_array(&result);
    assert_eq!(shuffled.len(), 1);
    assert_eq!(shuffled[0], serde_json::json!(42));
}

// ── rand.weighted_pick ───────────────────────────────────────────

#[test]
fn weighted_pick_returns_element() {
    let items = arr(&[
        serde_json::json!("a"),
        serde_json::json!("b"),
        serde_json::json!("c"),
    ]);
    let weights = arr(&[
        serde_json::json!(1),
        serde_json::json!(1),
        serde_json::json!(1),
    ]);
    for _ in 0..100 {
        let result = call("weighted_pick", &[items.clone(), weights.clone()]);
        let val = result.as_str().expect("expected string");
        assert!(val == "a" || val == "b" || val == "c");
    }
}

#[test]
fn weighted_pick_zero_weight_never_chosen() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(0), serde_json::json!(1)]);
    for _ in 0..200 {
        let result = call("weighted_pick", &[items.clone(), weights.clone()]);
        assert_eq!(result.as_str(), Some("b"));
    }
}

#[test]
fn weighted_pick_length_mismatch_errors() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(1)]);
    let result = call_err("weighted_pick", &[items, weights]);
    assert!(result.is_err());
}

#[test]
fn weighted_pick_all_zero_weights_errors() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(0), serde_json::json!(0)]);
    let result = call_err("weighted_pick", &[items, weights]);
    assert!(result.is_err());
}

#[test]
fn weighted_pick_empty_items_errors() {
    let items = arr(&[]);
    let weights = arr(&[]);
    let result = call_err("weighted_pick", &[items, weights]);
    assert!(result.is_err());
}

#[test]
fn weighted_pick_non_numeric_weight_errors() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(1), serde_json::json!("not a number")]);
    let result = call_err("weighted_pick", &[items, weights]);
    assert!(result.is_err());
}
