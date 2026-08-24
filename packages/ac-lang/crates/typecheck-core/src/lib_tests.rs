use crate::{Type, TypeEnv, TypeError, is_compatible};

#[test]
fn type_names_are_readable() {
    assert_eq!(Type::String.name(), "string");
    assert_eq!(Type::Number.name(), "number");
    assert_eq!(Type::Boolean.name(), "boolean");
    assert_eq!(Type::Null.name(), "null");
    assert_eq!(Type::Unknown.name(), "unknown");
    assert_eq!(Type::Never.name(), "never");
    assert_eq!(Type::Array(Box::new(Type::String)).name(), "array<string>");
}

#[test]
fn unknown_is_compatible_with_everything() {
    assert!(is_compatible(&Type::Unknown, &Type::String));
    assert!(is_compatible(&Type::Number, &Type::Unknown));
    assert!(is_compatible(&Type::Unknown, &Type::Unknown));
}

#[test]
fn never_is_compatible_with_everything() {
    assert!(is_compatible(&Type::Never, &Type::String));
    assert!(is_compatible(&Type::Number, &Type::Never));
}

#[test]
fn exact_matches_are_compatible() {
    assert!(is_compatible(&Type::String, &Type::String));
    assert!(is_compatible(&Type::Number, &Type::Number));
    assert!(!is_compatible(&Type::String, &Type::Number));
}

#[test]
fn array_compatibility_checks_item_type() {
    assert!(is_compatible(
        &Type::Array(Box::new(Type::String)),
        &Type::Array(Box::new(Type::String))
    ));
    assert!(!is_compatible(
        &Type::Array(Box::new(Type::String)),
        &Type::Array(Box::new(Type::Number))
    ));
}

#[test]
fn object_compatibility_allows_extra_properties() {
    let mut a = std::collections::BTreeMap::new();
    a.insert("name".to_string(), Type::String);
    a.insert("extra".to_string(), Type::Number);

    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    assert!(is_compatible(&Type::Object(a), &Type::Object(e)));
}

#[test]
fn object_incompatibility_on_missing_property() {
    let a = std::collections::BTreeMap::new();
    let mut e = std::collections::BTreeMap::new();
    e.insert("name".to_string(), Type::String);

    assert!(!is_compatible(&Type::Object(a), &Type::Object(e)));
}

#[test]
fn type_env_lookup_walks_scope_chain() {
    let mut parent = TypeEnv::new();
    parent.insert("name", Type::String);

    let mut child = TypeEnv::child(parent);
    child.insert("version", Type::Number);

    assert_eq!(child.lookup("name"), Some(&Type::String));
    assert_eq!(child.lookup("version"), Some(&Type::Number));
    assert_eq!(child.lookup("missing"), None);
}

#[test]
fn type_env_child_overrides_parent() {
    let mut parent = TypeEnv::new();
    parent.insert("mode", Type::String);

    let mut child = TypeEnv::child(parent);
    child.insert("mode", Type::Number);

    assert_eq!(child.lookup("mode"), Some(&Type::Number));
}

#[test]
fn type_env_all_bindings_merges_scopes() {
    let mut parent = TypeEnv::new();
    parent.insert("a", Type::String);
    parent.insert("b", Type::Number);

    let mut child = TypeEnv::child(parent);
    child.insert("b", Type::Boolean);
    child.insert("c", Type::Null);

    let all = child.all_bindings();
    assert_eq!(all.get("a"), Some(&Type::String));
    assert_eq!(all.get("b"), Some(&Type::Boolean));
    assert_eq!(all.get("c"), Some(&Type::Null));
}

#[test]
fn type_error_mismatch_formats_nicely() {
    let err = TypeError::mismatch("version", Type::Number, Type::String, "config");
    assert!(err.display().contains("version"));
    assert!(err.display().contains("number"));
    assert!(err.display().contains("string"));
}

#[test]
fn type_error_custom_message() {
    let err = TypeError::custom("mode", "field is immutable", "server");
    assert_eq!(err.expected, None);
    assert_eq!(err.actual, None);
    assert!(err.display().contains("immutable"));
}

#[test]
fn type_error_display_with_location() {
    let err = TypeError::mismatch("version", Type::Number, Type::String, "config");
    let displayed = err.display();
    assert!(
        !displayed.contains("["),
        "without location, no bracket prefix: {displayed}"
    );

    let mut err = err;
    err.location = Some(ac_source::SourceLocation::new(5, 10));
    let displayed = err.display();
    assert!(
        displayed.contains("[5:10]"),
        "with location, display should contain [5:10]: {displayed}"
    );
}