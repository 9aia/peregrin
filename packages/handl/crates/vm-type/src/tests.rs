use handl_typecheck_core::{FunctionType, Type, is_compatible as is_compatible_rust};

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
        return_type: Box::new(Type::Number),
    };
    let e_fn = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::String)],
        return_type: Box::new(Type::Number),
    };
    check(Type::Function(a_fn), Type::Function(e_fn));

    let e_fn2 = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::Number)],
        return_type: Box::new(Type::Number),
    };
    check(
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        }),
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

// ── handl A/B tests ───────────────────────────────────────────
// Compare eval_is_compatible_handl against the Rust reference.

fn check_handl(a: Type, e: Type) {
    let a_json = a.to_json();
    let e_json = e.to_json();
    let rust_result = is_compatible_rust(&a, &e);
    let handl_result = eval_is_compatible_handl(&a_json, &e_json)
        .unwrap_or_else(|err| panic!("handl eval failed for ({a:?}, {e:?}): {err:?}"));
    assert_eq!(
        rust_result, handl_result,
        "A/B mismatch: is_compatible({a:?}, {e:?}) — rust={rust_result}, handl={handl_result}"
    );
}

#[test]
fn handl_unknown_compatible_with_everything() {
    check_handl(Type::Unknown, Type::String);
    check_handl(Type::Number, Type::Unknown);
    check_handl(Type::Unknown, Type::Unknown);
}

#[test]
fn handl_never_compatible_with_everything() {
    check_handl(Type::Never, Type::String);
    check_handl(Type::Number, Type::Never);
}

#[test]
fn handl_exact_matches() {
    check_handl(Type::String, Type::String);
    check_handl(Type::Number, Type::Number);
    check_handl(Type::String, Type::Number);
}

#[test]
fn handl_array_compatibility() {
    check_handl(
        Type::Array(Box::new(Type::String)),
        Type::Array(Box::new(Type::String)),
    );
    check_handl(
        Type::Array(Box::new(Type::String)),
        Type::Array(Box::new(Type::Number)),
    );
}

#[test]
fn handl_object_allows_extra_properties() {
    let mut a = std::collections::BTreeMap::new();
    a.insert("name".to_string(), Type::String);
    a.insert("extra".to_string(), Type::Number);

    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    check_handl(Type::Object(a), Type::Object(e));
}

#[test]
fn handl_object_missing_property() {
    let a = std::collections::BTreeMap::new();
    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    check_handl(Type::Object(a), Type::Object(e));
}

#[test]
fn handl_var_compatibility() {
    check_handl(Type::Var("T".to_string()), Type::Var("T".to_string()));
    check_handl(Type::Var("T".to_string()), Type::Var("U".to_string()));
    check_handl(Type::Var("T".to_string()), Type::String);
}

#[test]
fn handl_trait_compatibility() {
    check_handl(
        Type::Trait("Numeric".to_string()),
        Type::Trait("Numeric".to_string()),
    );
    check_handl(
        Type::Trait("Numeric".to_string()),
        Type::Trait("Comparable".to_string()),
    );
}

#[test]
fn handl_function_compatibility() {
    let a_fn = FunctionType {
        type_params: vec![],
        params: vec![("x".to_string(), Type::String)],
        return_type: Box::new(Type::Number),
    };
    let e_fn = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::String)],
        return_type: Box::new(Type::Number),
    };
    check_handl(Type::Function(a_fn), Type::Function(e_fn));

    let e_fn2 = FunctionType {
        type_params: vec![],
        params: vec![("y".to_string(), Type::Number)],
        return_type: Box::new(Type::Number),
    };
    check_handl(
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        }),
        Type::Function(e_fn2),
    );
}

#[test]
fn handl_app_compatibility() {
    check_handl(
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
    );
    check_handl(
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::String]),
        Type::App(Box::new(Type::Var("F".to_string())), vec![Type::Number]),
    );
}
