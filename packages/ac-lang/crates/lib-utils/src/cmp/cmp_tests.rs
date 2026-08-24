use ac_runtime_modules::Module;
use ac_runtime_values::{CanonicalContext, Error, NonFunctionValue};

use super::CmpModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn boolean(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn arr(values: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(values.to_vec()))
}

fn obj(entries: &[(&str, serde_json::Value)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in entries {
        map.insert(k.to_string(), v.clone());
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

#[test]
fn module_name() {
    assert_eq!(CmpModule.name(), "cmp");
}

#[test]
fn unknown_function() {
    assert!(CmpModule.function("nope").is_none());
}

#[test]
fn eq_numbers() {
    assert!(call(&CmpModule, "eq", &[n(1.0), n(1.0)]).as_bool().unwrap());
}

#[test]
fn eq_strings() {
    assert!(call(&CmpModule, "eq", &[s("a"), s("a")]).as_bool().unwrap());
}

#[test]
fn eq_objects_deep() {
    assert!(call(
        &CmpModule,
        "eq",
        &[
            obj(&[("x", serde_json::json!(1.0))]),
            obj(&[("x", serde_json::json!(1.0))]),
        ]
    )
    .as_bool()
    .unwrap());
}

#[test]
fn ne_numbers() {
    assert!(call(&CmpModule, "ne", &[n(1.0), n(2.0)]).as_bool().unwrap());
}

#[test]
fn lt_numbers() {
    assert!(call(&CmpModule, "lt", &[n(1.0), n(2.0)]).as_bool().unwrap());
}

#[test]
fn lt_strings() {
    assert!(call(&CmpModule, "lt", &[s("a"), s("b")]).as_bool().unwrap());
}

#[test]
fn lt_mixed_types_number_before_string() {
    assert!(call(&CmpModule, "lt", &[n(1.0), s("a")]).as_bool().unwrap());
}

#[test]
fn le_equal() {
    assert!(call(&CmpModule, "le", &[n(2.0), n(2.0)]).as_bool().unwrap());
}

#[test]
fn gt_strings() {
    assert!(call(&CmpModule, "gt", &[s("b"), s("a")]).as_bool().unwrap());
}

#[test]
fn ge_mixed_types() {
    assert!(call(&CmpModule, "ge", &[s("a"), n(1.0)]).as_bool().unwrap());
}

#[test]
fn compare_less() {
    assert_eq!(
        call(&CmpModule, "compare", &[n(1.0), n(2.0)]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn compare_equal() {
    assert_eq!(
        call(&CmpModule, "compare", &[s("x"), s("x")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn compare_greater() {
    assert_eq!(
        call(&CmpModule, "compare", &[n(3.0), n(1.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn min_numbers() {
    assert_eq!(
        call(&CmpModule, "min", &[n(3.0), n(1.0), n(2.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn min_strings() {
    assert_eq!(
        call(&CmpModule, "min", &[s("c"), s("a"), s("b")]).as_str(),
        Some("a")
    );
}

#[test]
fn max_numbers() {
    assert_eq!(
        call(&CmpModule, "max", &[n(3.0), n(1.0), n(2.0)]).as_number(),
        Some(3.0)
    );
}

#[test]
fn max_strings() {
    assert_eq!(
        call(&CmpModule, "max", &[s("a"), s("c"), s("b")]).as_str(),
        Some("c")
    );
}

#[test]
fn clamp_within() {
    assert_eq!(
        call(&CmpModule, "clamp", &[n(5.0), n(0.0), n(10.0)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn clamp_below() {
    assert_eq!(
        call(&CmpModule, "clamp", &[n(-1.0), n(0.0), n(10.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn clamp_above() {
    assert_eq!(
        call(&CmpModule, "clamp", &[n(15.0), n(0.0), n(10.0)]).as_number(),
        Some(10.0)
    );
}

#[test]
fn clamp_inverted_bounds_errors() {
    let result = call_err(&CmpModule, "clamp", &[n(5.0), n(10.0), n(0.0)]);
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn between_true() {
    assert!(call(&CmpModule, "between", &[n(5.0), n(0.0), n(10.0)])
        .as_bool()
        .unwrap());
}

#[test]
fn between_false() {
    assert!(!call(&CmpModule, "between", &[n(15.0), n(0.0), n(10.0)])
        .as_bool()
        .unwrap());
}

#[test]
fn in_range_aliased() {
    assert!(call(&CmpModule, "in_range", &[n(0.0), n(-1.0), n(1.0)])
        .as_bool()
        .unwrap());
}

#[test]
fn is_sorted_numbers_asc() {
    assert!(call(
        &CmpModule,
        "is_sorted",
        &[arr(&[
            serde_json::json!(1.0),
            serde_json::json!(2.0),
            serde_json::json!(3.0)
        ])]
    )
    .as_bool()
    .unwrap());
}

#[test]
fn is_sorted_numbers_not() {
    assert!(!call(
        &CmpModule,
        "is_sorted",
        &[arr(&[
            serde_json::json!(3.0),
            serde_json::json!(1.0),
            serde_json::json!(2.0)
        ])]
    )
    .as_bool()
    .unwrap());
}

#[test]
fn is_sorted_desc() {
    assert!(call(
        &CmpModule,
        "is_sorted_desc",
        &[arr(&[
            serde_json::json!(3.0),
            serde_json::json!(2.0),
            serde_json::json!(1.0)
        ])]
    )
    .as_bool()
    .unwrap());
}

#[test]
fn sort_numbers_asc() {
    let result = call(
        &CmpModule,
        "sort",
        &[arr(&[
            serde_json::json!(3.0),
            serde_json::json!(1.0),
            serde_json::json!(2.0)
        ])],
    );
    assert_eq!(result.as_array().unwrap(), &[
        serde_json::json!(1.0),
        serde_json::json!(2.0),
        serde_json::json!(3.0)
    ]);
}

#[test]
fn sort_numbers_desc() {
    let result = call(
        &CmpModule,
        "sort_desc",
        &[arr(&[
            serde_json::json!(3.0),
            serde_json::json!(1.0),
            serde_json::json!(2.0)
        ])],
    );
    assert_eq!(result.as_array().unwrap(), &[
        serde_json::json!(3.0),
        serde_json::json!(2.0),
        serde_json::json!(1.0)
    ]);
}

#[test]
fn eq_missing_args_errors() {
    let result = call_err(&CmpModule, "eq", &[n(1.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn compare_missing_args_errors() {
    let result = call_err(&CmpModule, "compare", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn min_empty_args_errors() {
    let result = call_err(&CmpModule, "min", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}
