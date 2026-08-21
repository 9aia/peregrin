use super::test_helpers::*;

// ── rand.perlin_1d ──────────────────────────────────────────────

#[test]
fn perlin_1d_returns_finite() {
    let result = call("perlin_1d", &[n(0.5)]);
    let val = extract_num(&result);
    assert!(val.is_finite());
}

#[test]
fn perlin_1d_deterministic() {
    let a = call("perlin_1d", &[n(1.5)]);
    let b = call("perlin_1d", &[n(1.5)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn perlin_1d_in_range() {
    for i in 0..100 {
        let x = i as f64 * 0.1;
        let result = call("perlin_1d", &[n(x)]);
        let val = extract_num(&result);
        assert!((-1.5..=1.5).contains(&val), "1D noise {val} out of expected range at x={x}");
    }
}

#[test]
fn perlin_1d_no_args_errors() {
    let result = call_err("perlin_1d", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

// ── rand.perlin_2d ──────────────────────────────────────────────

#[test]
fn perlin_2d_returns_finite() {
    let result = call("perlin_2d", &[n(1.0), n(2.0)]);
    let val = extract_num(&result);
    assert!(val.is_finite());
}

#[test]
fn perlin_2d_deterministic() {
    let a = call("perlin_2d", &[n(0.5), n(0.5)]);
    let b = call("perlin_2d", &[n(0.5), n(0.5)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn perlin_2d_in_range() {
    for i in 0..50 {
        for j in 0..50 {
            let x = i as f64 * 0.1;
            let y = j as f64 * 0.1;
            let result = call("perlin_2d", &[n(x), n(y)]);
            let val = extract_num(&result);
            assert!((-1.5..=1.5).contains(&val), "2D noise {val} out of range at ({x},{y})");
        }
    }
}

#[test]
fn perlin_2d_integer_coords_zero() {
    let result = call("perlin_2d", &[n(0.0), n(0.0)]);
    let val = extract_num(&result);
    assert!(val.abs() < 1e-10, "noise at integer coords should be ~0, got {val}");
}

#[test]
fn perlin_2d_no_args_errors() {
    let result = call_err("perlin_2d", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

#[test]
fn perlin_2d_one_arg_errors() {
    let result = call_err("perlin_2d", &[n(1.0)]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

// ── rand.perlin_3d ──────────────────────────────────────────────

#[test]
fn perlin_3d_returns_finite() {
    let result = call("perlin_3d", &[n(0.1), n(0.2), n(0.3)]);
    let val = extract_num(&result);
    assert!(val.is_finite());
}

#[test]
fn perlin_3d_deterministic() {
    let a = call("perlin_3d", &[n(0.1), n(0.2), n(0.3)]);
    let b = call("perlin_3d", &[n(0.1), n(0.2), n(0.3)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn perlin_3d_in_range() {
    for i in 0..20 {
        for j in 0..20 {
            for k in 0..20 {
                let x = i as f64 * 0.1;
                let y = j as f64 * 0.1;
                let z = k as f64 * 0.1;
                let result = call("perlin_3d", &[n(x), n(y), n(z)]);
                let val = extract_num(&result);
                assert!((-1.5..=1.5).contains(&val), "3D noise {val} out of range at ({x},{y},{z})");
            }
        }
    }
}

#[test]
fn perlin_3d_no_args_errors() {
    let result = call_err("perlin_3d", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

#[test]
fn perlin_3d_one_arg_errors() {
    let result = call_err("perlin_3d", &[n(1.0)]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

// ── rand.simplex_2d ──────────────────────────────────────────────

#[test]
fn simplex_2d_returns_finite() {
    let result = call("simplex_2d", &[n(1.0), n(2.0)]);
    let val = extract_num(&result);
    assert!(val.is_finite());
}

#[test]
fn simplex_2d_deterministic() {
    let a = call("simplex_2d", &[n(0.5), n(0.5)]);
    let b = call("simplex_2d", &[n(0.5), n(0.5)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn simplex_2d_in_range() {
    for i in 0..50 {
        for j in 0..50 {
            let x = i as f64 * 0.1;
            let y = j as f64 * 0.1;
            let result = call("simplex_2d", &[n(x), n(y)]);
            let val = extract_num(&result);
            assert!((-1.0..=1.0).contains(&val), "2D simplex noise {val} out of range at ({x},{y})");
        }
    }
}

#[test]
fn simplex_2d_origin_zero() {
    let result = call("simplex_2d", &[n(0.0), n(0.0)]);
    let val = extract_num(&result);
    assert!(val.abs() < 1e-10, "simplex at origin should be ~0, got {val}");
}

#[test]
fn simplex_2d_no_args_errors() {
    let result = call_err("simplex_2d", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

#[test]
fn simplex_2d_one_arg_errors() {
    let result = call_err("simplex_2d", &[n(1.0)]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

// ── rand.simplex_3d ──────────────────────────────────────────────

#[test]
fn simplex_3d_returns_finite() {
    let result = call("simplex_3d", &[n(0.1), n(0.2), n(0.3)]);
    let val = extract_num(&result);
    assert!(val.is_finite());
}

#[test]
fn simplex_3d_deterministic() {
    let a = call("simplex_3d", &[n(0.1), n(0.2), n(0.3)]);
    let b = call("simplex_3d", &[n(0.1), n(0.2), n(0.3)]);
    assert_eq!(extract_num(&a), extract_num(&b));
}

#[test]
fn simplex_3d_in_range() {
    for i in 0..20 {
        for j in 0..20 {
            for k in 0..20 {
                let x = i as f64 * 0.1;
                let y = j as f64 * 0.1;
                let z = k as f64 * 0.1;
                let result = call("simplex_3d", &[n(x), n(y), n(z)]);
                let val = extract_num(&result);
                assert!((-1.0..=1.0).contains(&val), "3D simplex noise {val} out of range at ({x},{y},{z})");
            }
        }
    }
}

#[test]
fn simplex_3d_origin_zero() {
    let result = call("simplex_3d", &[n(0.0), n(0.0), n(0.0)]);
    let val = extract_num(&result);
    assert!(val.abs() < 1e-10, "simplex at origin should be ~0, got {val}");
}

#[test]
fn simplex_3d_no_args_errors() {
    let result = call_err("simplex_3d", &[]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}

#[test]
fn simplex_3d_one_arg_errors() {
    let result = call_err("simplex_3d", &[n(1.0)]);
    assert!(matches!(result, Err(ac_vm::Error::ArgumentCount { .. })));
}
