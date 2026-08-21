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

// ── rand.exponential ─────────────────────────────────────────────

#[test]
fn exponential_returns_finite() {
    for _ in 0..100 {
        let result = call("exponential", &[n(2.0)]);
        let val = extract_num(&result);
        assert!(val.is_finite(), "expected finite number, got {val}");
    }
}

#[test]
fn exponential_non_negative() {
    for _ in 0..1000 {
        let result = call("exponential", &[n(1.0)]);
        let val = extract_num(&result);
        assert!(val >= 0.0, "exponential must be non-negative, got {val}");
    }
}

#[test]
fn exponential_mean_convergence() {
    let lambda = 2.0;
    let expected_mean = 1.0 / lambda;
    let mut sum = 0.0;
    let count = 5000;
    for _ in 0..count {
        let result = call("exponential", &[n(lambda)]);
        sum += extract_num(&result);
    }
    let sample_mean = sum / count as f64;
    assert!(
        (sample_mean - expected_mean).abs() < 0.1,
        "sample mean {sample_mean} too far from expected {expected_mean}"
    );
}

#[test]
fn exponential_zero_lambda_errors() {
    let result = call_err("exponential", &[n(0.0)]);
    assert!(result.is_err());
}

#[test]
fn exponential_negative_lambda_errors() {
    let result = call_err("exponential", &[n(-1.0)]);
    assert!(result.is_err());
}

#[test]
fn exponential_no_args_errors() {
    let result = call_err("exponential", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

// ── rand.poisson ─────────────────────────────────────────────────

#[test]
fn poisson_returns_non_negative_integer() {
    for _ in 0..100 {
        let result = call("poisson", &[n(3.0)]);
        let val = extract_num(&result);
        assert!(val >= 0.0, "poisson must be non-negative, got {val}");
        assert_eq!(val, val.trunc(), "poisson must be an integer, got {val}");
    }
}

#[test]
fn poisson_mean_convergence() {
    let lambda = 5.0;
    let mut sum = 0.0;
    let count = 5000;
    for _ in 0..count {
        let result = call("poisson", &[n(lambda)]);
        sum += extract_num(&result);
    }
    let sample_mean = sum / count as f64;
    assert!(
        (sample_mean - lambda).abs() < 0.5,
        "sample mean {sample_mean} too far from lambda {lambda}"
    );
}

#[test]
fn poisson_zero_lambda_returns_zero() {
    let result = call("poisson", &[n(0.0)]);
    assert_eq!(extract_num(&result), 0.0);
}

#[test]
fn poisson_negative_lambda_errors() {
    let result = call_err("poisson", &[n(-1.0)]);
    assert!(result.is_err());
}

#[test]
fn poisson_no_args_errors() {
    let result = call_err("poisson", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

// ── rand.binomial ────────────────────────────────────────────────

#[test]
fn binomial_in_range() {
    for _ in 0..100 {
        let result = call("binomial", &[n(10.0), n(0.5)]);
        let val = extract_num(&result) as i64;
        assert!((0..=10).contains(&val), "binomial must be in [0, n], got {val}");
    }
}

#[test]
fn binomial_zero_trials_returns_zero() {
    let result = call("binomial", &[n(0.0), n(0.5)]);
    assert_eq!(extract_num(&result), 0.0);
}

#[test]
fn binomial_p_zero_returns_zero() {
    let result = call("binomial", &[n(10.0), n(0.0)]);
    assert_eq!(extract_num(&result), 0.0);
}

#[test]
fn binomial_p_one_returns_n() {
    let result = call("binomial", &[n(10.0), n(1.0)]);
    assert_eq!(extract_num(&result), 10.0);
}

#[test]
fn binomial_mean_convergence() {
    let n_trials = 20.0;
    let p = 0.3;
    let expected_mean = n_trials * p;
    let mut sum = 0.0;
    let count = 5000;
    for _ in 0..count {
        let result = call("binomial", &[n(n_trials), n(p)]);
        sum += extract_num(&result);
    }
    let sample_mean = sum / count as f64;
    assert!(
        (sample_mean - expected_mean).abs() < 1.0,
        "sample mean {sample_mean} too far from expected {expected_mean}"
    );
}

#[test]
fn binomial_negative_trials_errors() {
    let result = call_err("binomial", &[n(-1.0), n(0.5)]);
    assert!(result.is_err());
}

#[test]
fn binomial_p_out_of_range_errors() {
    let result = call_err("binomial", &[n(10.0), n(1.5)]);
    assert!(result.is_err());
    let result = call_err("binomial", &[n(10.0), n(-0.1)]);
    assert!(result.is_err());
}

#[test]
fn binomial_no_args_errors() {
    let result = call_err("binomial", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}
