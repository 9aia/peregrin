use super::*;
use ac_dialect_canonical::AC_DIALECT;
use ac_typecheck_core::{FunctionType, Type, TypeEnv};

#[test]
fn parse_string_literal() {
    let ast = parse_expr("\"hello\"", &AC_DIALECT).unwrap();
    assert_eq!(ast, ExprAst::Literal(Type::String));
}

#[test]
fn parse_number_literal() {
    let ast = parse_expr("123", &AC_DIALECT).unwrap();
    assert_eq!(ast, ExprAst::Literal(Type::Number));
}

#[test]
fn parse_boolean_literal() {
    assert_eq!(
        parse_expr("true", &AC_DIALECT).unwrap(),
        ExprAst::Literal(Type::Boolean)
    );
    assert_eq!(
        parse_expr("false", &AC_DIALECT).unwrap(),
        ExprAst::Literal(Type::Boolean)
    );
}

#[test]
fn parse_null_literal() {
    assert_eq!(
        parse_expr("null", &AC_DIALECT).unwrap(),
        ExprAst::Literal(Type::Null)
    );
}

#[test]
fn parse_path() {
    let ast = parse_expr("doc.name", &AC_DIALECT).unwrap();
    assert_eq!(ast, ExprAst::Path("doc.name".to_string()));
}

#[test]
fn parse_shorthand_path() {
    let ast = parse_expr("name", &AC_DIALECT).unwrap();
    assert_eq!(ast, ExprAst::Path("name".to_string()));
}

#[test]
fn parse_function_call() {
    let ast = parse_expr("add(1, 2)", &AC_DIALECT).unwrap();
    assert_eq!(
        ast,
        ExprAst::Call(
            "add".to_string(),
            vec![
                ExprAst::Literal(Type::Number),
                ExprAst::Literal(Type::Number)
            ]
        )
    );
}

#[test]
fn parse_nested_function_call() {
    let ast = parse_expr("add(mul(2, 3), 4)", &AC_DIALECT).unwrap();
    assert_eq!(
        ast,
        ExprAst::Call(
            "add".to_string(),
            vec![
                ExprAst::Call(
                    "mul".to_string(),
                    vec![
                        ExprAst::Literal(Type::Number),
                        ExprAst::Literal(Type::Number)
                    ]
                ),
                ExprAst::Literal(Type::Number),
            ]
        )
    );
}

#[test]
fn parse_dollar_wrapped() {
    let ast = parse_expr("${add(1, 2)}", &AC_DIALECT).unwrap();
    assert_eq!(
        ast,
        ExprAst::Call(
            "add".to_string(),
            vec![
                ExprAst::Literal(Type::Number),
                ExprAst::Literal(Type::Number)
            ]
        )
    );
}

#[test]
fn infer_literal() {
    let env = TypeEnv::new();
    let funcs = FunctionTable::new();
    assert_eq!(infer("\"hello\"", &env, &funcs, &AC_DIALECT), Type::String);
    assert_eq!(infer("42", &env, &funcs, &AC_DIALECT), Type::Number);
    assert_eq!(infer("true", &env, &funcs, &AC_DIALECT), Type::Boolean);
}

#[test]
fn infer_path_lookup() {
    let mut env = TypeEnv::new();
    env.insert("name", Type::String);
    env.insert("version", Type::Number);

    let funcs = FunctionTable::new();
    assert_eq!(infer("name", &env, &funcs, &AC_DIALECT), Type::String);
    assert_eq!(infer("version", &env, &funcs, &AC_DIALECT), Type::Number);
}

#[test]
fn infer_unknown_path() {
    let env = TypeEnv::new();
    let funcs = FunctionTable::new();
    assert_eq!(infer("missing", &env, &funcs, &AC_DIALECT), Type::Unknown);
}

#[test]
fn infer_function_call() {
    let mut env = TypeEnv::new();
    env.insert("x", Type::Number);
    env.insert("y", Type::Number);

    let mut funcs = FunctionTable::new();
    funcs.insert(
        "add",
        FunctionType {
            params: vec![
                ("x".to_string(), Type::Number),
                ("y".to_string(), Type::Number),
            ],
            return_type: Box::new(Type::Number),
        },
    );

    assert_eq!(infer("add(1, 2)", &env, &funcs, &AC_DIALECT), Type::Number);
    assert_eq!(infer("add(x, y)", &env, &funcs, &AC_DIALECT), Type::Number);
}

#[test]
fn infer_nested_function_call() {
    let mut env = TypeEnv::new();
    env.insert("x", Type::Number);

    let mut funcs = FunctionTable::new();
    funcs.insert(
        "add",
        FunctionType {
            params: vec![
                ("a".to_string(), Type::Number),
                ("b".to_string(), Type::Number),
            ],
            return_type: Box::new(Type::Number),
        },
    );

    assert_eq!(
        infer("add(add(1, 2), x)", &env, &funcs, &AC_DIALECT),
        Type::Number
    );
}

#[test]
fn infer_unknown_function() {
    let env = TypeEnv::new();
    let funcs = FunctionTable::new();
    assert_eq!(
        infer("unknown(1)", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

#[test]
fn infer_arity_mismatch() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "add",
        FunctionType {
            params: vec![
                ("a".to_string(), Type::Number),
                ("b".to_string(), Type::Number),
            ],
            return_type: Box::new(Type::Number),
        },
    );

    let env = TypeEnv::new();
    assert_eq!(infer("add(1)", &env, &funcs, &AC_DIALECT), Type::Unknown);
}

#[test]
fn infer_type_mismatch_in_args() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "add",
        FunctionType {
            params: vec![
                ("a".to_string(), Type::Number),
                ("b".to_string(), Type::Number),
            ],
            return_type: Box::new(Type::Number),
        },
    );

    let env = TypeEnv::new();
    assert_eq!(
        infer("add(1, \"hello\")", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

#[test]
fn infer_dotted_path_into_object() {
    let mut env = TypeEnv::new();

    let mut doc_props = std::collections::BTreeMap::new();
    doc_props.insert("name".to_string(), Type::String);
    doc_props.insert("version".to_string(), Type::Number);
    env.insert("doc", Type::Object(doc_props));

    let funcs = FunctionTable::new();
    assert_eq!(infer("doc.name", &env, &funcs, &AC_DIALECT), Type::String);
    assert_eq!(
        infer("doc.version", &env, &funcs, &AC_DIALECT),
        Type::Number
    );
    assert_eq!(
        infer("doc.missing", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

#[test]
fn infer_shorthand_resolves_via_doc() {
    let mut env = TypeEnv::new();

    let mut doc_props = std::collections::BTreeMap::new();
    doc_props.insert("name".to_string(), Type::String);
    env.insert("doc", Type::Object(doc_props));

    let funcs = FunctionTable::new();
    assert_eq!(infer("name", &env, &funcs, &AC_DIALECT), Type::String);
}

#[test]
fn parse_unmatched_open_paren() {
    assert!(parse_expr("(", &AC_DIALECT).is_some());
    assert!(parse_expr("foo(", &AC_DIALECT).is_some());
    assert!(parse_expr("foo(bar", &AC_DIALECT).is_some());
}

#[test]
fn parse_empty_function_name() {
    let ast = parse_expr("(x)", &AC_DIALECT).unwrap();
    assert_eq!(ast, ExprAst::Path("(x)".to_string()));
}

#[test]
fn infer_call_function_typed_binding() {
    let mut env = TypeEnv::new();
    env.insert(
        "f",
        Type::Function(FunctionType {
            params: vec![("x".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        }),
    );

    let funcs = FunctionTable::new();
    assert_eq!(infer("f(42)", &env, &funcs, &AC_DIALECT), Type::Number);
}

#[test]
fn infer_call_function_typed_field_via_shorthand() {
    let mut env = TypeEnv::new();
    let mut doc_props = std::collections::BTreeMap::new();
    doc_props.insert(
        "transform".to_string(),
        Type::Function(FunctionType {
            params: vec![("x".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        }),
    );
    env.insert("doc", Type::Object(doc_props));

    let funcs = FunctionTable::new();
    assert_eq!(
        infer("transform(5)", &env, &funcs, &AC_DIALECT),
        Type::Number
    );
}

#[test]
fn infer_call_function_typed_arg_mismatch() {
    let mut env = TypeEnv::new();
    env.insert(
        "f",
        Type::Function(FunctionType {
            params: vec![("x".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        }),
    );

    let funcs = FunctionTable::new();
    assert_eq!(
        infer("f(\"hello\")", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

#[test]
fn infer_call_function_typed_arity_mismatch() {
    let mut env = TypeEnv::new();
    env.insert(
        "f",
        Type::Function(FunctionType {
            params: vec![("x".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        }),
    );

    let funcs = FunctionTable::new();
    assert_eq!(infer("f(1, 2)", &env, &funcs, &AC_DIALECT), Type::Unknown);
}

#[test]
fn infer_call_non_function_binding() {
    let mut env = TypeEnv::new();
    env.insert("f", Type::Number);

    let funcs = FunctionTable::new();
    assert_eq!(infer("f(1)", &env, &funcs, &AC_DIALECT), Type::Unknown);
}
