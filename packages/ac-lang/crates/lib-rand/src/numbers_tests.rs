use super::test_helpers::*;

// ── rand.int ─────────────────────────────────────────────────────

#[test]
fn int_in_range() {
    for _ in 0..100 {
        let result = call("int", &[n(1.0), n(6.0)]);
        let val = extract_num(&result) as i64;
        assert!((1..=6).contains(&val), "got {val}, expected 1..=6");
    }
}

#[test]
fn int_single_value_range() {
    let result = call("int", &[n(5.0), n(5.0)]);
    assert_eq!(extract_num(&result) as i64, 5);
}

#[test]
fn int_negative_range() {
    for _ in 0..100 {
        let result = call("int", &[n(-10.0), n(-1.0)]);
        let val = extract_num(&result) as i64;
        assert!((-10..=-1).contains(&val), "got {val}, expected -10..=-1");
    }
}

#[test]
fn int_min_greater_than_max_errors() {
    let result = call_err("int", &[n(10.0), n(1.0)]);
    assert!(result.is_err());
}

#[test]
fn int_no_args_errors() {
    let result = call_err("int", &[]);
    assert!(matches!(
        result,
        Err(ac_runtime::Error::ArgumentCount { .. })
    ));
}

#[test]
fn int_non_integer_errors() {
    let result = call_err("int", &[n(1.5), n(6.0)]);
    assert!(matches!(
        result,
        Err(ac_runtime::Error::TypeMismatch { .. })
    ));
}

#[test]
fn int_single_arg() {
    for _ in 0..100 {
        let result = call("int", &[n(6.0)]);
        let val = extract_num(&result) as i64;
        assert!((0..=6).contains(&val), "got {val}, expected 0..=6");
    }
}

#[test]
fn int_single_arg_zero() {
    let result = call("int", &[n(0.0)]);
    assert_eq!(extract_num(&result) as i64, 0);
}

#[test]
fn int_three_args_errors() {
    let result = call_err("int", &[n(1.0), n(2.0), n(3.0)]);
    assert!(matches!(
        result,
        Err(ac_runtime::Error::ArgumentCount { .. })
    ));
}

// ── rand.float ───────────────────────────────────────────────────

#[test]
fn float_unit_range() {
    for _ in 0..100 {
        let result = call("float", &[]);
        let val = extract_num(&result);
        assert!((0.0..1.0).contains(&val), "got {val}, expected [0, 1)");
    }
}

#[test]
fn float_with_range() {
    for _ in 0..100 {
        let result = call("float", &[n(10.0), n(20.0)]);
        let val = extract_num(&result);
        assert!((10.0..20.0).contains(&val), "got {val}, expected [10, 20)");
    }
}

#[test]
fn float_single_arg() {
    for _ in 0..100 {
        let result = call("float", &[n(10.0)]);
        let val = extract_num(&result);
        assert!((0.0..10.0).contains(&val), "got {val}, expected [0, 10)");
    }
}

#[test]
fn float_single_arg_zero_errors() {
    let result = call_err("float", &[n(0.0)]);
    assert!(result.is_err());
}

#[test]
fn float_single_arg_negative_errors() {
    let result = call_err("float", &[n(-5.0)]);
    assert!(result.is_err());
}

#[test]
fn float_min_greater_than_max_errors() {
    let result = call_err("float", &[n(20.0), n(10.0)]);
    assert!(result.is_err());
}

#[test]
fn float_min_equals_max() {
    let result = call("float", &[n(5.0), n(5.0)]);
    assert_eq!(extract_num(&result), 5.0);
}

// ── rand.bool ────────────────────────────────────────────────────

#[test]
fn bool_returns_boolean() {
    for _ in 0..100 {
        let result = call("bool", &[]);
        let val = result.as_bool();
        assert!(val.is_some(), "expected a boolean value");
    }
}

#[test]
fn bool_eventually_returns_both() {
    let mut got_true = false;
    let mut got_false = false;
    for _ in 0..200 {
        let result = call("bool", &[]);
        match result.as_bool() {
            Some(true) => got_true = true,
            Some(false) => got_false = true,
            None => panic!("expected boolean"),
        }
    }
    assert!(
        got_true && got_false,
        "expected both true and false in 200 tries"
    );
}