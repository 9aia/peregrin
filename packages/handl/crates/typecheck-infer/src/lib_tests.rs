use super::*;
use handl_dialect_canonical::AC_DIALECT;
use handl_typecheck_core::{FunctionType, Type, TypeEnv, TypeParam};

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

// ── handle expressions ──────────────────────────────────────

#[test]
fn parse_handle_basic() {
    let ast = parse_expr(
        "handle(fetch(\"data\")) { \"fetch\"(url) as k: k(42) }",
        &AC_DIALECT,
    )
    .unwrap();
    match ast {
        ExprAst::Handle { body, handlers } => {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].op_name, "fetch");
            assert_eq!(handlers[0].params, vec!["url"]);
            assert_eq!(handlers[0].continuation_name, "k");
            match *body {
                ExprAst::Call(ref name, _) => assert_eq!(name, "fetch"),
                ref other => panic!("expected Call body, got {other:?}"),
            }
        }
        other => panic!("expected Handle, got {other:?}"),
    }
}

#[test]
fn parse_handle_multiple_handlers() {
    let ast = parse_expr(
        "handle(add(fetch(\"d\"), log(\"m\"))) { \"fetch\"(url) as k: k(42), \"log\"(msg) as k: k(1) }",
        &AC_DIALECT,
    )
    .unwrap();
    match ast {
        ExprAst::Handle { handlers, .. } => {
            assert_eq!(handlers.len(), 2);
            assert_eq!(handlers[0].op_name, "fetch");
            assert_eq!(handlers[1].op_name, "log");
        }
        other => panic!("expected Handle, got {other:?}"),
    }
}

#[test]
fn parse_handle_no_params() {
    let ast = parse_expr(
        "handle(fetch()) { \"fetch\"() as k: \"cached\" }",
        &AC_DIALECT,
    )
    .unwrap();
    match ast {
        ExprAst::Handle { handlers, .. } => {
            assert_eq!(handlers.len(), 1);
            assert!(handlers[0].params.is_empty());
        }
        other => panic!("expected Handle, got {other:?}"),
    }
}

#[test]
fn parse_handle_dollar_wrapped() {
    let ast = parse_expr(
        "${handle(fetch(\"d\")) { \"fetch\"(url) as k: k(42) }}",
        &AC_DIALECT,
    )
    .unwrap();
    assert!(matches!(ast, ExprAst::Handle { .. }));
}

#[test]
fn parse_handle_handler_body_is_call() {
    let ast = parse_expr(
        "handle(fetch(\"data\")) { \"fetch\"(url) as k: k(42) }",
        &AC_DIALECT,
    )
    .unwrap();
    match ast {
        ExprAst::Handle { handlers, .. } => match &*handlers[0].body {
            ExprAst::Call(name, _) => assert_eq!(name, "k"),
            other => panic!("expected Call handler body, got {other:?}"),
        },
        other => panic!("expected Handle, got {other:?}"),
    }
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
            type_params: vec![],
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
            type_params: vec![],
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
            type_params: vec![],
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
            type_params: vec![],
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
            type_params: vec![],
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
            type_params: vec![],
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
            type_params: vec![],
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
            type_params: vec![],
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

// ── generic function inference ───────────────────────────────

#[test]
fn infer_generic_identity() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "identity",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Var("T".to_string())),
        },
    );

    let env = TypeEnv::new();
    assert_eq!(
        infer("identity(\"hello\")", &env, &funcs, &AC_DIALECT),
        Type::String
    );
    assert_eq!(
        infer("identity(42)", &env, &funcs, &AC_DIALECT),
        Type::Number
    );
}

#[test]
fn infer_generic_map() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "map",
        FunctionType {
            type_params: vec![TypeParam::new("T"), TypeParam::new("U")],
            params: vec![
                (
                    "arr".to_string(),
                    Type::Array(Box::new(Type::Var("T".to_string()))),
                ),
                (
                    "fn".to_string(),
                    Type::Function(FunctionType {
                        type_params: vec![],
                        params: vec![("x".to_string(), Type::Var("T".to_string()))],
                        return_type: Box::new(Type::Var("U".to_string())),
                    }),
                ),
            ],
            return_type: Box::new(Type::Array(Box::new(Type::Var("U".to_string())))),
        },
    );

    let mut env = TypeEnv::new();
    env.insert("strs", Type::Array(Box::new(Type::String)));
    let mut doc_props = std::collections::BTreeMap::new();
    doc_props.insert(
        "to_len".to_string(),
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("s".to_string(), Type::String)],
            return_type: Box::new(Type::Number),
        }),
    );
    env.insert("doc", Type::Object(doc_props));

    assert_eq!(
        infer("map(strs, to_len)", &env, &funcs, &AC_DIALECT),
        Type::Array(Box::new(Type::Number))
    );
}

#[test]
fn infer_generic_with_unknown_arg() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "identity",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Var("T".to_string())),
        },
    );

    let env = TypeEnv::new();
    // unknown_path resolves to Unknown → T binds to Unknown → return Unknown
    assert_eq!(
        infer("identity(missing)", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

#[test]
fn infer_generic_type_mismatch() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "first",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![(
                "arr".to_string(),
                Type::Array(Box::new(Type::Var("T".to_string()))),
            )],
            return_type: Box::new(Type::Var("T".to_string())),
        },
    );

    let env = TypeEnv::new();
    // "hello" is String, not Array<T> → unification fails → Unknown
    assert_eq!(
        infer("first(\"hello\")", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

#[test]
fn infer_generic_consistency_check() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "apply_fn",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![
                ("x".to_string(), Type::Var("T".to_string())),
                (
                    "f".to_string(),
                    Type::Function(FunctionType {
                        type_params: vec![],
                        params: vec![("y".to_string(), Type::Var("T".to_string()))],
                        return_type: Box::new(Type::Var("T".to_string())),
                    }),
                ),
            ],
            return_type: Box::new(Type::Var("T".to_string())),
        },
    );

    let mut env = TypeEnv::new();
    env.insert("str_val", Type::String);
    let mut doc_props = std::collections::BTreeMap::new();
    doc_props.insert(
        "num_fn".to_string(),
        Type::Function(FunctionType {
            type_params: vec![],
            params: vec![("y".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        }),
    );
    env.insert("doc", Type::Object(doc_props));

    // T=String from first arg, but f expects T=Number → inconsistent → Unknown
    assert_eq!(
        infer("apply_fn(str_val, num_fn)", &env, &funcs, &AC_DIALECT),
        Type::Unknown
    );
}

// ── check_call_detailed tests ──────────────────────────────────

#[test]
fn detailed_monomorphic_arg_count_mismatch() {
    let fn_type = FunctionType {
        type_params: vec![],
        params: vec![("x".to_string(), Type::Number)],
        return_type: Box::new(Type::Number),
    };
    let result = check_call_detailed("double", &fn_type, &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.function, "double");
    assert!(matches!(
        err.kind,
        CallErrorKind::ArgCount {
            expected: 1,
            got: 0
        }
    ));
}

#[test]
fn detailed_monomorphic_arg_type_mismatch() {
    let fn_type = FunctionType {
        type_params: vec![],
        params: vec![("x".to_string(), Type::Number)],
        return_type: Box::new(Type::Number),
    };
    let result = check_call_detailed("double", &fn_type, &[Type::String]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.function, "double");
    assert!(matches!(
        err.kind,
        CallErrorKind::ArgType {
            param,
            index: 0,
            ref expected,
            ref actual} if param == "x" && *expected == Type::Number && *actual == Type::String
    ));
}

#[test]
fn detailed_generic_unification_failure() {
    let fn_type = FunctionType {
        type_params: vec![TypeParam::new("T")],
        params: vec![(
            "arr".to_string(),
            Type::Array(Box::new(Type::Var("T".to_string()))),
        )],
        return_type: Box::new(Type::Var("T".to_string())),
    };
    // Pass String where Array<T> is expected → unification fails
    let result = check_call_detailed("first", &fn_type, &[Type::String]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.function, "first");
    assert!(matches!(
        err.kind,
        CallErrorKind::UnificationFailed {
            param,
            index: 0,
            ..
        } if param == "arr"
    ));
}

#[test]
fn detailed_generic_success_returns_ok() {
    let fn_type = FunctionType {
        type_params: vec![TypeParam::new("T")],
        params: vec![("x".to_string(), Type::Var("T".to_string()))],
        return_type: Box::new(Type::Var("T".to_string())),
    };
    let result = check_call_detailed("identity", &fn_type, &[Type::String]);
    assert_eq!(result, Ok(Type::String));
}

#[test]
fn detailed_monomorphic_success_returns_ok() {
    let fn_type = FunctionType {
        type_params: vec![],
        params: vec![("x".to_string(), Type::Number)],
        return_type: Box::new(Type::Number),
    };
    let result = check_call_detailed("double", &fn_type, &[Type::Number]);
    assert_eq!(result, Ok(Type::Number));
}

#[test]
fn detailed_call_error_to_type_error_message() {
    let err = CallCheckError {
        function: "identity".to_string(),
        kind: CallErrorKind::ArgCount {
            expected: 1,
            got: 0,
        },
    };
    let type_err = err.to_type_error();
    assert_eq!(type_err.field, "identity");
    assert!(type_err.message.contains("expected 1 argument(s), got 0"));
}

#[test]
fn detailed_generic_unification_error_message() {
    let err = CallCheckError {
        function: "first".to_string(),
        kind: CallErrorKind::UnificationFailed {
            param: "arr".to_string(),
            index: 0,
            expected: Type::Array(Box::new(Type::Var("T".to_string()))),
            actual: Type::String,
        },
    };
    let type_err = err.to_type_error();
    assert!(type_err.message.contains("first"));
    assert!(
        type_err
            .message
            .contains("type mismatch in generic parameter")
    );
    assert!(type_err.message.contains("arr"));
}

// ── infer_ast_detailed tests ───────────────────────────────────

#[test]
fn infer_detailed_collects_generic_unification_error() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "first",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![(
                "arr".to_string(),
                Type::Array(Box::new(Type::Var("T".to_string()))),
            )],
            return_type: Box::new(Type::Var("T".to_string())),
        },
    );

    let env = TypeEnv::new();
    let ast = parse_expr("first(\"hello\")", &AC_DIALECT).unwrap();
    let (ty, errors) = infer_ast_detailed(&ast, &env, &funcs, &AC_DIALECT);

    assert_eq!(ty, Type::Unknown);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("first"));
    assert!(errors[0].message.contains("type mismatch"));
}

#[test]
fn infer_detailed_no_errors_on_valid_generic_call() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "identity",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Var("T".to_string())),
        },
    );

    let env = TypeEnv::new();
    let ast = parse_expr("identity(\"hello\")", &AC_DIALECT).unwrap();
    let (ty, errors) = infer_ast_detailed(&ast, &env, &funcs, &AC_DIALECT);

    assert_eq!(ty, Type::String);
    assert!(errors.is_empty());
}

#[test]
fn infer_detailed_monomorphic_arg_count_error() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "double",
        FunctionType {
            type_params: vec![],
            params: vec![("x".to_string(), Type::Number)],
            return_type: Box::new(Type::Number),
        },
    );

    let env = TypeEnv::new();
    let ast = parse_expr("double()", &AC_DIALECT).unwrap();
    let (ty, errors) = infer_ast_detailed(&ast, &env, &funcs, &AC_DIALECT);

    assert_eq!(ty, Type::Unknown);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("expected 1 argument(s), got 0"));
}

// ── return-type-driven inference tests ──────────────────────────

#[test]
fn return_type_driven_binds_unbound_type_var() {
    let mut funcs = FunctionTable::new();
    // make_array<T>() -> Array<T>  — T appears only in return type
    funcs.insert(
        "make_array",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![],
            return_type: Box::new(Type::Array(Box::new(Type::Var("T".to_string())))),
        },
    );

    let env = TypeEnv::new();
    // Without expected type: T is unbound → Unknown
    assert_eq!(
        infer("make_array()", &env, &funcs, &AC_DIALECT),
        Type::Array(Box::new(Type::Unknown))
    );

    // With expected type Array<String>: T binds to String
    let ast = parse_expr("make_array()", &AC_DIALECT).unwrap();
    let ty = infer_ast_with_expected(
        &ast,
        &env,
        &funcs,
        &AC_DIALECT,
        Some(&Type::Array(Box::new(Type::String))),
    );
    assert_eq!(ty, Type::Array(Box::new(Type::String)));
}

#[test]
fn return_type_driven_with_argument_binding() {
    let mut funcs = FunctionTable::new();
    // wrap<T>(x: T) -> Array<T>  — T appears in both param and return
    funcs.insert(
        "wrap",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![("x".to_string(), Type::Var("T".to_string()))],
            return_type: Box::new(Type::Array(Box::new(Type::Var("T".to_string())))),
        },
    );

    let env = TypeEnv::new();
    // Argument binds T=String, expected type is consistent
    let ast = parse_expr("wrap(\"hello\")", &AC_DIALECT).unwrap();
    let ty = infer_ast_with_expected(
        &ast,
        &env,
        &funcs,
        &AC_DIALECT,
        Some(&Type::Array(Box::new(Type::String))),
    );
    assert_eq!(ty, Type::Array(Box::new(Type::String)));
}

#[test]
fn return_type_driven_mismatch_does_not_fail() {
    let mut funcs = FunctionTable::new();
    funcs.insert(
        "make_array",
        FunctionType {
            type_params: vec![TypeParam::new("T")],
            params: vec![],
            return_type: Box::new(Type::Array(Box::new(Type::Var("T".to_string())))),
        },
    );

    let env = TypeEnv::new();
    // Expected type is String (not Array<...>) — return-type unification
    // fails gracefully, result is Array<Unknown>
    let ast = parse_expr("make_array()", &AC_DIALECT).unwrap();
    let ty = infer_ast_with_expected(&ast, &env, &funcs, &AC_DIALECT, Some(&Type::String));
    assert_eq!(ty, Type::Array(Box::new(Type::Unknown)));
}
