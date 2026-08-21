use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::RandModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn arr(values: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(values.to_vec()))
}

fn extract_num(v: &NonFunctionValue) -> f64 {
    v.as_number().expect("expected number")
}

fn extract_str(v: &NonFunctionValue) -> String {
    v.as_str().expect("expected string").to_string()
}

fn extract_array(v: &NonFunctionValue) -> Vec<serde_json::Value> {
    v.as_array().expect("expected array").to_vec()
}

// ── rand.int ─────────────────────────────────────────────────────

#[test]
fn int_in_range() {
    for _ in 0..100 {
        let result = call(&RandModule, "int", &[n(1.0), n(6.0)]);
        let val = extract_num(&result) as i64;
        assert!((1..=6).contains(&val), "got {val}, expected 1..=6");
    }
}

#[test]
fn int_single_value_range() {
    let result = call(&RandModule, "int", &[n(5.0), n(5.0)]);
    assert_eq!(extract_num(&result) as i64, 5);
}

#[test]
fn int_negative_range() {
    for _ in 0..100 {
        let result = call(&RandModule, "int", &[n(-10.0), n(-1.0)]);
        let val = extract_num(&result) as i64;
        assert!((-10..=-1).contains(&val), "got {val}, expected -10..=-1");
    }
}

#[test]
fn int_min_greater_than_max_errors() {
    let result = call_err(&RandModule, "int", &[n(10.0), n(1.0)]);
    assert!(result.is_err());
}

#[test]
fn int_no_args_errors() {
    let result = call_err(&RandModule, "int", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn int_non_integer_errors() {
    let result = call_err(&RandModule, "int", &[n(1.5), n(6.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── rand.float ───────────────────────────────────────────────────

#[test]
fn float_unit_range() {
    for _ in 0..100 {
        let result = call(&RandModule, "float", &[]);
        let val = extract_num(&result);
        assert!(val >= 0.0 && val < 1.0, "got {val}, expected [0, 1)");
    }
}

#[test]
fn float_with_range() {
    for _ in 0..100 {
        let result = call(&RandModule, "float", &[n(10.0), n(20.0)]);
        let val = extract_num(&result);
        assert!(val >= 10.0 && val < 20.0, "got {val}, expected [10, 20)");
    }
}

#[test]
fn float_one_arg_errors() {
    let result = call_err(&RandModule, "float", &[n(1.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn float_min_greater_than_max_errors() {
    let result = call_err(&RandModule, "float", &[n(20.0), n(10.0)]);
    assert!(result.is_err());
}

// ── rand.bool ────────────────────────────────────────────────────

#[test]
fn bool_returns_boolean() {
    for _ in 0..100 {
        let result = call(&RandModule, "bool", &[]);
        let val = result.as_bool();
        assert!(val.is_some(), "expected a boolean value");
    }
}

#[test]
fn bool_eventually_returns_both() {
    let mut got_true = false;
    let mut got_false = false;
    for _ in 0..200 {
        let result = call(&RandModule, "bool", &[]);
        match result.as_bool() {
            Some(true) => got_true = true,
            Some(false) => got_false = true,
            None => panic!("expected boolean"),
        }
    }
    assert!(got_true && got_false, "expected both true and false in 200 tries");
}

// ── rand.pick ────────────────────────────────────────────────────

#[test]
fn pick_returns_element_from_array() {
    let items = arr(&[
        serde_json::json!("a"),
        serde_json::json!("b"),
        serde_json::json!("c"),
    ]);
    for _ in 0..100 {
        let result = call(&RandModule, "pick", &[items.clone()]);
        let val = result.as_str().expect("expected string");
        assert!(val == "a" || val == "b" || val == "c");
    }
}

#[test]
fn pick_empty_array_errors() {
    let result = call_err(&RandModule, "pick", &[arr(&[])]);
    assert!(result.is_err());
}

#[test]
fn pick_no_args_errors() {
    let result = call_err(&RandModule, "pick", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
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
    let result = call(&RandModule, "sample", &[items, n(3.0)]);
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
        let result = call(&RandModule, "sample", &[items.clone(), n(5.0)]);
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
    let result = call(&RandModule, "sample", &[items, n(0.0)]);
    let arr = extract_array(&result);
    assert!(arr.is_empty());
}

#[test]
fn sample_count_exceeds_length_errors() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call_err(&RandModule, "sample", &[items, n(5.0)]);
    assert!(result.is_err());
}

#[test]
fn sample_negative_count_errors() {
    let items = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = call_err(&RandModule, "sample", &[items, n(-1.0)]);
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
    let result = call(&RandModule, "shuffle", &[items.clone()]);
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
    let result = call(&RandModule, "shuffle", &[arr(&[])]);
    assert!(extract_array(&result).is_empty());
}

#[test]
fn shuffle_single_element() {
    let result = call(&RandModule, "shuffle", &[arr(&[serde_json::json!(42)])]);
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
    let weights = arr(&[serde_json::json!(1), serde_json::json!(1), serde_json::json!(1)]);
    for _ in 0..100 {
        let result = call(&RandModule, "weighted_pick", &[items.clone(), weights.clone()]);
        let val = result.as_str().expect("expected string");
        assert!(val == "a" || val == "b" || val == "c");
    }
}

#[test]
fn weighted_pick_zero_weight_never_chosen() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(0), serde_json::json!(1)]);
    for _ in 0..200 {
        let result = call(&RandModule, "weighted_pick", &[items.clone(), weights.clone()]);
        assert_eq!(result.as_str(), Some("b"));
    }
}

#[test]
fn weighted_pick_length_mismatch_errors() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(1)]);
    let result = call_err(&RandModule, "weighted_pick", &[items, weights]);
    assert!(result.is_err());
}

#[test]
fn weighted_pick_all_zero_weights_errors() {
    let items = arr(&[serde_json::json!("a"), serde_json::json!("b")]);
    let weights = arr(&[serde_json::json!(0), serde_json::json!(0)]);
    let result = call_err(&RandModule, "weighted_pick", &[items, weights]);
    assert!(result.is_err());
}

#[test]
fn weighted_pick_empty_items_errors() {
    let items = arr(&[]);
    let weights = arr(&[]);
    let result = call_err(&RandModule, "weighted_pick", &[items, weights]);
    assert!(result.is_err());
}

// ── rand.string ──────────────────────────────────────────────────

#[test]
fn string_correct_length() {
    for len in [1, 8, 16, 32, 64] {
        let result = call(&RandModule, "string", &[n(len as f64)]);
        let s = extract_str(&result);
        assert_eq!(s.len(), len as usize, "expected length {len}");
    }
}

#[test]
fn string_zero_length() {
    let result = call(&RandModule, "string", &[n(0.0)]);
    assert_eq!(extract_str(&result), "");
}

#[test]
fn string_alphanumeric_only() {
    let result = call(&RandModule, "string", &[n(100.0)]);
    let s = extract_str(&result);
    for c in s.chars() {
        assert!(
            c.is_ascii_alphanumeric(),
            "non-alphanumeric character: {c}"
        );
    }
}

#[test]
fn string_negative_length_errors() {
    let result = call_err(&RandModule, "string", &[n(-1.0)]);
    assert!(result.is_err());
}

// ── rand.bytes ───────────────────────────────────────────────────

#[test]
fn bytes_correct_length() {
    for len in [1, 8, 16, 32] {
        let result = call(&RandModule, "bytes", &[n(len as f64)]);
        let arr = extract_array(&result);
        assert_eq!(arr.len(), len as usize);
    }
}

#[test]
fn bytes_zero_length() {
    let result = call(&RandModule, "bytes", &[n(0.0)]);
    assert!(extract_array(&result).is_empty());
}

#[test]
fn bytes_in_range() {
    let result = call(&RandModule, "bytes", &[n(100.0)]);
    let arr = extract_array(&result);
    for v in &arr {
        let byte_val = v.as_f64().unwrap() as u8;
        assert_eq!(byte_val as f64, v.as_f64().unwrap());
    }
}

#[test]
fn bytes_negative_length_errors() {
    let result = call_err(&RandModule, "bytes", &[n(-1.0)]);
    assert!(result.is_err());
}

// ── rand.uuid ────────────────────────────────────────────────────

#[test]
fn uuid_format() {
    let result = call(&RandModule, "uuid", &[]);
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
        let result = call(&RandModule, "uuid", &[]);
        let uuid = extract_str(&result);
        let version_char = uuid.chars().nth(14).unwrap();
        assert_eq!(version_char, '4', "version nibble must be 4, got {version_char}");
    }
}

#[test]
fn uuid_variant() {
    for _ in 0..50 {
        let result = call(&RandModule, "uuid", &[]);
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
        let result = call(&RandModule, "uuid", &[]);
        let uuid = extract_str(&result);
        assert!(seen.insert(uuid), "duplicate UUID generated");
    }
}

#[test]
fn uuid_hex_only() {
    let result = call(&RandModule, "uuid", &[]);
    let uuid = extract_str(&result);
    for c in uuid.chars().filter(|&c| c != '-') {
        assert!(c.is_ascii_hexdigit(), "non-hex character: {c}");
    }
}

// ── module structure ─────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(RandModule.name(), "rand");
}

#[test]
fn all_functions_registered() {
    let names = [
        "int",
        "float",
        "bool",
        "pick",
        "sample",
        "shuffle",
        "weighted_pick",
        "string",
        "bytes",
        "uuid",
    ];
    for name in &names {
        assert!(
            RandModule.function(name).is_some(),
            "function '{name}' should be registered"
        );
    }
}

#[test]
fn unknown_function_returns_none() {
    assert!(RandModule.function("nonexistent").is_none());
}

// ── VM integration ───────────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(RandModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_int() {
    let expr = Expr::call("rand.int", vec![Expr::value(n(1.0)), Expr::value(n(10.0))]);
    let result = eval_expr(&expr);
    let val = extract_num(&result) as i64;
    assert!((1..=10).contains(&val));
}

#[test]
fn vm_uuid() {
    let expr = Expr::call("rand.uuid", vec![]);
    let result = eval_expr(&expr);
    let uuid = extract_str(&result);
    assert_eq!(uuid.len(), 36);
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(RandModule);
    assert!(host.function("rand.int").is_some());
    assert!(host.function("rand.float").is_some());
    assert!(host.function("rand.bool").is_some());
    assert!(host.function("rand.pick").is_some());
    assert!(host.function("rand.sample").is_some());
    assert!(host.function("rand.shuffle").is_some());
    assert!(host.function("rand.weighted_pick").is_some());
    assert!(host.function("rand.string").is_some());
    assert!(host.function("rand.bytes").is_some());
    assert!(host.function("rand.uuid").is_some());
}
