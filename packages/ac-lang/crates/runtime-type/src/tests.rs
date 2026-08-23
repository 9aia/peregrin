use ac_typecheck_core::{FunctionType, Type, is_compatible as is_compatible_rust};

use crate::*;

fn check(a: Type, e: Type) {
    let a_json = a.to_json();
    let e_json = e.to_json();
    let rust_result = is_compatible_rust(&a, &e);
    let json_result = is_compatible_json(&a_json, &e_json);
    assert_eq!(
        rust_result, json_result,
        "mismatch: is_compatible({:?}, {:?}) — rust={}, json={}",
        a, e, rust_result, json_result
    );
}

#[test]
fn unknown_compatible_with_everything() {
    check(Type::Unknown, Type::String);
    check(Type::Number, Type::Unknown);
    check(Type::Unknown, Type::Unknown);
}

#[test]
fn never_compatible_with_everything() {
    check(Type::Never, Type::String);
    check(Type::Number, Type::Never);
}

#[test]
fn exact_matches() {
    check(Type::String, Type::String);
    check(Type::Number, Type::Number);
    check(Type::String, Type::Number);
}

#[test]
fn array_compatibility() {
    check(
        Type::Array(Box::new(Type::String)),
        Type::Array(Box::new(Type::String)),
    );
    check(
        Type::Array(Box::new(Type::String)),
        Type::Array(Box::new(Type::Number)),
    );
}

#[test]
fn object_allows_extra_properties() {
    let mut a = std::collections::BTreeMap::new();
    a.insert("name".to_string(), Type::String);
    a.insert("extra".to_string(), Type::Number);

    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    check(Type::Object(a), Type::Object(e));
}

#[test]
fn object_missing_property() {
    let a = std::collections::BTreeMap::new();
    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    check(Type::Object(a), Type::Object(e));
}

#[test]
fn var_compatibility() {
    check(Type::Var("T".to_string()), Type::Var("T".to_string()));
    check(Type::Var("T".to_string()), Type::Var("U".to_string()));
    check(Type::Var("T".to_string()), Type::String);
}

#[test]
fn trait_compatibility() {
    check(
        Type::Trait("Numeric".to_string()),
        Type::Trait("Numeric".to_string()),
    );
    check(
        Type::Trait("Numeric".to_string()),
        Type::Trait("Comparable".to_string()),
    );
}

#[test]
fn function_compatibility() {
    let a_fn = FunctionType {
        type_params: vec![],
        params: vec![("x".to_string(), Type::String)],
        return_type: Box::new(Type::Number)};
    let e_fn = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::String)],
        return_type: Box::new(Type::Number)};
    check(Type::Function(a_fn), Type::Function(e_fn));

    let e_fn2 = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::Number)],
        return_type: Box::new(Type::Number)};
    check(
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number)}),
        Type::Function(e_fn2),
    );
}

#[test]
fn app_compatibility() {
    check(
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
    );
    check(
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::Number]),
    );
}

#[test]
fn to_json_roundtrip() {
    let types = vec![
        Type::String,
        Type::Number,
        Type::Boolean,
        Type::Null,
        Type::Unknown,
        Type::Never,
        Type::Var("T".to_string()),
        Type::Trait("Numeric".to_string()),
        Type::Array(Box::new(Type::String)),
        Type::App(
            Box::new(Type::Var("F".to_string())),
            vec![Type::String, Type::Number],
        ),
    ];
    for ty in &types {
        let json = ty.to_json();
        let recovered = Type::from_json(&json).unwrap();
        assert_eq!(ty, &recovered, "roundtrip failed for {:?}", ty);
    }
}

// ── AC-Lang A/B tests ───────────────────────────────────────────
// Compare eval_is_compatible_ac_lang against the Rust reference.

fn check_ac_lang(a: Type, e: Type) {
    let a_json = a.to_json();
    let e_json = e.to_json();
    let rust_result = is_compatible_rust(&a, &e);
    let ac_lang_result = eval_is_compatible_ac_lang(&a_json, &e_json)
        .unwrap_or_else(|err| panic!("AC-Lang eval failed for ({a:?}, {e:?}): {err:?}"));
    assert_eq!(
        rust_result, ac_lang_result,
        "A/B mismatch: is_compatible({a:?}, {e:?}) — rust={rust_result}, ac_lang={ac_lang_result}"
    );
}

#[test]
fn ac_lang_unknown_compatible_with_everything() {
    check_ac_lang(Type::Unknown, Type::String);
    check_ac_lang(Type::Number, Type::Unknown);
    check_ac_lang(Type::Unknown, Type::Unknown);
}

#[test]
fn ac_lang_never_compatible_with_everything() {
    check_ac_lang(Type::Never, Type::String);
    check_ac_lang(Type::Number, Type::Never);
}

#[test]
fn ac_lang_exact_matches() {
    check_ac_lang(Type::String, Type::String);
    check_ac_lang(Type::Number, Type::Number);
    check_ac_lang(Type::String, Type::Number);
}

#[test]
fn ac_lang_array_compatibility() {
    check_ac_lang(
        Type::Array(Box::new(Type::String)),
        Type::Array(Box::new(Type::String)),
    );
    check_ac_lang(
        Type::Array(Box::new(Type::String)),
        Type::Array(Box::new(Type::Number)),
    );
}

#[test]
fn ac_lang_object_allows_extra_properties() {
    let mut a = std::collections::BTreeMap::new();
    a.insert("name".to_string(), Type::String);
    a.insert("extra".to_string(), Type::Number);

    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    check_ac_lang(Type::Object(a), Type::Object(e));
}

#[test]
fn ac_lang_object_missing_property() {
    let a = std::collections::BTreeMap::new();
    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    check_ac_lang(Type::Object(a), Type::Object(e));
}

#[test]
fn ac_lang_var_compatibility() {
    check_ac_lang(Type::Var("T".to_string()), Type::Var("T".to_string()));
    check_ac_lang(Type::Var("T".to_string()), Type::Var("U".to_string()));
    check_ac_lang(Type::Var("T".to_string()), Type::String);
}

#[test]
fn ac_lang_trait_compatibility() {
    check_ac_lang(
        Type::Trait("Numeric".to_string()),
        Type::Trait("Numeric".to_string()),
    );
    check_ac_lang(
        Type::Trait("Numeric".to_string()),
        Type::Trait("Comparable".to_string()),
    );
}

#[test]
fn ac_lang_function_compatibility() {
    let a_fn = FunctionType {
        type_params: vec![],
        params: vec![("x".to_string(), Type::String)],
        return_type: Box::new(Type::Number)};
    let e_fn = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::String)],
        return_type: Box::new(Type::Number)};
    check_ac_lang(Type::Function(a_fn), Type::Function(e_fn));

    let e_fn2 = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::Number)],
        return_type: Box::new(Type::Number)};
    check_ac_lang(
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number)}),
        Type::Function(e_fn2),
    );
}

#[test]
fn ac_lang_app_compatibility() {
    check_ac_lang(
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
    );
    check_ac_lang(
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::Number]),
    );
}
