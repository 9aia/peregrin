use ac_ast_common::TypeFactory;

use crate::{CanonicalTypeFactory, Type};

#[test]
fn type_factory_primitive() {
    let f = CanonicalTypeFactory;
    assert_eq!(f.primitive("string"), Type::primitive("string"));
}

#[test]
fn type_factory_array() {
    let f = CanonicalTypeFactory;
    assert_eq!(f.array(f.primitive("number")), Type::array(Type::primitive("number")));
}

#[test]
fn type_factory_object() {
    let f = CanonicalTypeFactory;
    assert_eq!(
        f.object(vec![("x".to_string(), f.primitive("number"))]),
        Type::object(vec![("x".to_string(), Type::primitive("number"))]),
    );
}

#[test]
fn type_factory_function() {
    let f = CanonicalTypeFactory;
    let t = f.function(vec![f.primitive("number")], f.primitive("string"));
    assert_eq!(
        t,
        Type::function(vec![Type::primitive("number")], Type::primitive("string")),
    );
}
