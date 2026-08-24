use ac_runtime::Function;
use ac_runtime_values::{CanonicalContext, NonFunctionValue};
use serde_json::Value as JsonValue;

use crate::mood::{MoodCnd, MoodInd, MoodInt, MoodNeg, MoodNorm, MoodPot};

fn ctx() -> CanonicalContext {
    CanonicalContext::new()
}

fn ok(v: JsonValue) -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Object(serde_json::Map::from_iter([(
        "ok".to_string(),
        v,
    )])))
}

fn err(v: JsonValue) -> NonFunctionValue {
    NonFunctionValue::new(JsonValue::Object(serde_json::Map::from_iter([(
        "err".to_string(),
        v,
    )])))
}

fn call(f: &dyn Function<NonFunctionValue>, args: &[NonFunctionValue]) -> NonFunctionValue {
    let mut c = ctx();
    f.call(args, &mut c).unwrap()
}

#[test]
fn int_matches_variant() {
    assert!(
        call(&MoodInt, &[ok(JsonValue::from(42)), NonFunctionValue::new("ok")])
            .as_bool()
            .unwrap()
    );
    assert!(
        !call(
            &MoodInt,
            &[
                err(JsonValue::String("bad".to_string())),
                NonFunctionValue::new("ok")
            ]
        )
        .as_bool()
        .unwrap()
    );
}

#[test]
fn int_matches_type() {
    assert!(
        call(
            &MoodInt,
            &[
                NonFunctionValue::new("hello"),
                NonFunctionValue::new("type.string")
            ]
        )
        .as_bool()
        .unwrap()
    );
    assert!(
        !call(
            &MoodInt,
            &[
                NonFunctionValue::new(42.0),
                NonFunctionValue::new("type.string")
            ]
        )
        .as_bool()
        .unwrap()
    );
}

#[test]
fn pot_extracts_ok() {
    let result = call(&MoodPot, &[ok(JsonValue::from(42)), NonFunctionValue::new("ok")]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn pot_returns_null_on_mismatch() {
    let result = call(&MoodPot, &[err(JsonValue::from("bad")), NonFunctionValue::new("ok")]);
    assert!(result.is_null());
}

#[test]
fn neg_extracts_err() {
    let result = call(&MoodNeg, &[err(JsonValue::from("bad")), NonFunctionValue::new("ok")]);
    assert_eq!(result.as_str(), Some("bad"));
}

#[test]
fn neg_returns_null_when_matches() {
    let result = call(&MoodNeg, &[ok(JsonValue::from(42)), NonFunctionValue::new("ok")]);
    assert!(result.is_null());
}

#[test]
fn neg_keeps_non_matching_value() {
    let result = call(&MoodNeg, &[NonFunctionValue::new(5.0), NonFunctionValue::new("type.string")]);
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn norm_wraps_result() {
    let ok_result = call(&MoodNorm, &[ok(JsonValue::from(42)), NonFunctionValue::new("ok")]);
    assert_eq!(ok_result.get("ok").unwrap().as_f64(), Some(42.0));

    let err_result = call(&MoodNorm, &[err(JsonValue::from("bad")), NonFunctionValue::new("ok")]);
    assert!(err_result.get("err").is_some());
}

#[test]
fn cnd_falls_back() {
    let value = call(&MoodCnd, &[
        err(JsonValue::from("bad")),
        NonFunctionValue::new("ok"),
        NonFunctionValue::new("default"),
    ]);
    assert_eq!(value.as_str(), Some("default"));
}

#[test]
fn ind_strict() {
    let value = call(&MoodInd, &[ok(JsonValue::from(42)), NonFunctionValue::new("ok")]);
    assert_eq!(value.as_number(), Some(42.0));
}
