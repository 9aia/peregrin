use super::test_helpers::*;

// ── rand.gauss ───────────────────────────────────────────────────

#[test]
fn gauss_returns_finite() {
    for _ in 0..100 {
        let result = call("gauss", &[n(0.0), n(1.0)]);
        let val = extract_num(&result);
        assert!(val.is_finite(), "expected finite number, got {val}");
    }
}

#[test]
fn gauss_mean_shift() {
    let mean = 100.0;
    let stddev = 1.0;
    let mut sum = 0.0;
    let count = 1000;
    for _ in 0..count {
        let result = call("gauss", &[n(mean), n(stddev)]);
        sum += extract_num(&result);
    }
    let sample_mean = sum / count as f64;
    assert!((sample_mean - mean).abs() < 10.0, "sample mean {sample_mean} too far from {mean}");
}

#[test]
fn gauss_zero_stddev_returns_mean() {
    let result = call("gauss", &[n(42.0), n(0.0)]);
    assert_eq!(extract_num(&result), 42.0);
}

#[test]
fn gauss_negative_stddev_errors() {
    let result = call_err("gauss", &[n(0.0), n(-1.0)]);
    assert!(result.is_err());
}

#[test]
fn gauss_no_args_errors() {
    let result = call_err("gauss", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}
