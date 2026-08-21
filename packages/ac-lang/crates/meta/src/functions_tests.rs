use super::*;
use ac_runtime::RuntimeValue;
use ac_vm::{Context, Eval, Vm};
use ac_ast::Expr;

// ── typeof ──────────────────────────────────────────────────

#[test]
fn typeof_string() {
    let v = call_meta("typeof", &[RuntimeValue::new("hello")]);
    assert_eq!(v.as_str(), Some("string"));
}

#[test]
fn typeof_number() {
    let v = call_meta("typeof", &[RuntimeValue::new(42)]);
    assert_eq!(v.as_str(), Some("number"));
}

#[test]
fn typeof_boolean() {
    let v = call_meta("typeof", &[RuntimeValue::new(true)]);
    assert_eq!(v.as_str(), Some("boolean"));
}

#[test]
fn typeof_null() {
    let v = call_meta("typeof", &[RuntimeValue::new(serde_json::Value::Null)]);
    assert_eq!(v.as_str(), Some("null"));
}

#[test]
fn typeof_array() {
    let v = call_meta("typeof", &[RuntimeValue::new(vec![1, 2, 3])]);
    assert_eq!(v.as_str(), Some("array"));
}

#[test]
fn typeof_object() {
    let v = call_meta(
        "typeof",
        &[RuntimeValue::new(serde_json::json!({"key": "value"}))],
    );
    assert_eq!(v.as_str(), Some("object"));
}

#[test]
fn typeof_function() {
    let fn_val = serde_json::json!({
        "params": [{"name": "x", "schema": {"type": "number"}}],
        "body": "mul(x, 2)"
    });
    let v = call_meta("typeof", &[RuntimeValue::new(fn_val)]);
    assert_eq!(v.as_str(), Some("function"));
}

// ── keys ────────────────────────────────────────────────────

#[test]
fn keys_of_object() {
    let v = call_meta(
        "keys",
        &[RuntimeValue::new(serde_json::json!({"a": 1, "b": 2, "c": 3}))],
    );
    let arr = v.as_array().unwrap();
    let names: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[test]
fn keys_of_non_object_errors() {
    let result = try_call_meta("keys", &[RuntimeValue::new(42)]);
    assert!(result.is_err());
}

// ── is_function ─────────────────────────────────────────────

#[test]
fn is_function_true_for_function_value() {
    let fn_val = serde_json::json!({
        "body": "mul(x, 2)"
    });
    let v = call_meta("is_function", &[RuntimeValue::new(fn_val)]);
    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn is_function_false_for_plain_object() {
    let v = call_meta(
        "is_function",
        &[RuntimeValue::new(serde_json::json!({"key": "value"}))],
    );
    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn is_function_false_for_string() {
    let v = call_meta("is_function", &[RuntimeValue::new("hello")]);
    assert_eq!(v.as_bool(), Some(false));
}

// ── eval ────────────────────────────────────────────────────

#[test]
fn eval_with_callback() {
    struct SimpleEval;
    impl Eval for SimpleEval {
        fn eval_expr(
            &self,
            _expr: &str,
            vars: &[(String, RuntimeValue)],
        ) -> Result<RuntimeValue, ac_vm::Error> {
            // Simple: just return the first var's value doubled
            let val = vars
                .first()
                .map(|(_, v)| v.as_number().unwrap_or(0.0))
                .unwrap_or(0.0);
            Ok(RuntimeValue::new(val * 2.0))
        }
    }

    let mut vm = Vm::new(MetaHost::default());
    let mut ctx = Context::new();
    ctx.set_eval(Box::new(SimpleEval));
    ctx.define("x", RuntimeValue::new(21));

    let expr = Expr::call(
        "eval",
        vec![Expr::value(RuntimeValue::new("mul(x, 2)"))],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn eval_without_callback_errors() {
    let mut vm = Vm::new(MetaHost::default());
    let mut ctx = Context::new();

    let expr = Expr::call(
        "eval",
        vec![Expr::value(RuntimeValue::new("add(1, 2)"))],
    );
    let result = vm.eval(&expr, &mut ctx);
    assert!(result.is_err());
}

#[test]
fn eval_non_string_errors() {
    let mut vm = Vm::new(MetaHost::default());
    let mut ctx = Context::new();

    let expr = Expr::call("eval", vec![Expr::value(RuntimeValue::new(42))]);
    let result = vm.eval(&expr, &mut ctx);
    assert!(result.is_err());
}

// ── parse ───────────────────────────────────────────────────

#[test]
fn parse_string_literal() {
    let v = call_meta("parse", &[RuntimeValue::new("\"hello\"")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("literal"));
    assert_eq!(obj.get("type").unwrap().as_str(), Some("string"));
    assert_eq!(obj.get("value").unwrap().as_str(), Some("hello"));
}

#[test]
fn parse_number_literal() {
    let v = call_meta("parse", &[RuntimeValue::new("42")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("literal"));
    assert_eq!(obj.get("type").unwrap().as_str(), Some("number"));
    assert_eq!(obj.get("value").unwrap().as_f64().unwrap(), 42.0);
}

#[test]
fn parse_boolean_literal() {
    let v = call_meta("parse", &[RuntimeValue::new("true")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("literal"));
    assert_eq!(obj.get("type").unwrap().as_str(), Some("boolean"));
    assert_eq!(obj.get("value").unwrap().as_bool(), Some(true));
}

#[test]
fn parse_null_literal() {
    let v = call_meta("parse", &[RuntimeValue::new("null")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("literal"));
    assert_eq!(obj.get("type").unwrap().as_str(), Some("null"));
    assert!(obj.get("value").unwrap().is_null());
}

#[test]
fn parse_path() {
    let v = call_meta("parse", &[RuntimeValue::new("doc.name")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("path"));
    assert_eq!(obj.get("path").unwrap().as_str(), Some("doc.name"));
}

#[test]
fn parse_function_call() {
    let v = call_meta("parse", &[RuntimeValue::new("add(1, 2)")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("call"));
    assert_eq!(obj.get("name").unwrap().as_str(), Some("add"));
    let args = obj.get("args").unwrap().as_array().unwrap();
    assert_eq!(args.len(), 2);
}

#[test]
fn parse_nested_function_call() {
    let v = call_meta("parse", &[RuntimeValue::new("add(mul(2, 3), 4)")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("call"));
    assert_eq!(obj.get("name").unwrap().as_str(), Some("add"));
    let args = obj.get("args").unwrap().as_array().unwrap();
    assert_eq!(args.len(), 2);
    let first = args[0].as_object().unwrap();
    assert_eq!(first.get("kind").unwrap().as_str(), Some("call"));
    assert_eq!(first.get("name").unwrap().as_str(), Some("mul"));
}

#[test]
fn parse_dollar_wrapped() {
    let v = call_meta("parse", &[RuntimeValue::new("${add(1, 2)}")]);
    let obj = v.as_object().unwrap();
    assert_eq!(obj.get("kind").unwrap().as_str(), Some("call"));
    assert_eq!(obj.get("name").unwrap().as_str(), Some("add"));
}

// ── ast_to_string ───────────────────────────────────────────

#[test]
fn ast_to_string_literal() {
    let ast = serde_json::json!({
        "kind": "literal",
        "value": "hello",
        "type": "string"
    });
    let v = call_meta("ast_to_string", &[RuntimeValue::new(ast)]);
    let s = v.as_str().unwrap();
    assert!(s == "\"hello\"" || s == "'hello'");
}

#[test]
fn ast_to_string_number() {
    let ast = serde_json::json!({
        "kind": "literal",
        "value": 42,
        "type": "number"
    });
    let v = call_meta("ast_to_string", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_str(), Some("42"));
}

#[test]
fn ast_to_string_path() {
    let ast = serde_json::json!({
        "kind": "path",
        "path": "doc.name"
    });
    let v = call_meta("ast_to_string", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_str(), Some("doc.name"));
}

#[test]
fn ast_to_string_call() {
    let ast = serde_json::json!({
        "kind": "call",
        "name": "add",
        "args": [
            {"kind": "literal", "value": 1, "type": "number"},
            {"kind": "literal", "value": 2, "type": "number"}
        ]
    });
    let v = call_meta("ast_to_string", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_str(), Some("add(1, 2)"));
}

#[test]
fn ast_to_string_nested_call() {
    let ast = serde_json::json!({
        "kind": "call",
        "name": "add",
        "args": [
            {"kind": "call", "name": "mul", "args": [
                {"kind": "literal", "value": 2, "type": "number"},
                {"kind": "literal", "value": 3, "type": "number"}
            ]},
            {"kind": "literal", "value": 4, "type": "number"}
        ]
    });
    let v = call_meta("ast_to_string", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_str(), Some("add(mul(2, 3), 4)"));
}

// ── ast_name ────────────────────────────────────────────────

#[test]
fn ast_name_of_call() {
    let ast = serde_json::json!({"kind": "call", "name": "add", "args": []});
    let v = call_meta("ast_name", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_str(), Some("add"));
}

#[test]
fn ast_name_of_non_call_returns_null() {
    let ast = serde_json::json!({"kind": "path", "path": "doc.name"});
    let v = call_meta("ast_name", &[RuntimeValue::new(ast)]);
    assert!(v.is_null());
}

// ── ast_args ────────────────────────────────────────────────

#[test]
fn ast_args_of_call() {
    let ast = serde_json::json!({
        "kind": "call",
        "name": "add",
        "args": [
            {"kind": "literal", "value": 1, "type": "number"},
            {"kind": "literal", "value": 2, "type": "number"}
        ]
    });
    let v = call_meta("ast_args", &[RuntimeValue::new(ast)]);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn ast_args_of_non_call_returns_empty() {
    let ast = serde_json::json!({"kind": "path", "path": "doc.name"});
    let v = call_meta("ast_args", &[RuntimeValue::new(ast)]);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 0);
}

// ── ast_is_call ─────────────────────────────────────────────

#[test]
fn ast_is_call_true() {
    let ast = serde_json::json!({"kind": "call", "name": "add", "args": []});
    let v = call_meta("ast_is_call", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn ast_is_call_false_for_path() {
    let ast = serde_json::json!({"kind": "path", "path": "doc.name"});
    let v = call_meta("ast_is_call", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn ast_is_call_false_for_literal() {
    let ast = serde_json::json!({"kind": "literal", "value": 42, "type": "number"});
    let v = call_meta("ast_is_call", &[RuntimeValue::new(ast)]);
    assert_eq!(v.as_bool(), Some(false));
}

// ── round-trip: parse → ast_to_string ───────────────────────

#[test]
fn round_trip_simple_call() {
    let original = "add(1, 2)";
    let parsed = call_meta("parse", &[RuntimeValue::new(original)]);
    let roundtrip = call_meta("ast_to_string", &[parsed]);
    assert_eq!(roundtrip.as_str(), Some(original));
}

#[test]
fn round_trip_nested_call() {
    let original = "add(mul(2, 3), 4)";
    let parsed = call_meta("parse", &[RuntimeValue::new(original)]);
    let roundtrip = call_meta("ast_to_string", &[parsed]);
    assert_eq!(roundtrip.as_str(), Some(original));
}

#[test]
fn round_trip_string_literal() {
    let original = "\"hello\"";
    let parsed = call_meta("parse", &[RuntimeValue::new(original)]);
    let roundtrip = call_meta("ast_to_string", &[parsed]);
    assert_eq!(roundtrip.as_str(), Some(original));
}

#[test]
fn round_trip_path() {
    let original = "doc.name";
    let parsed = call_meta("parse", &[RuntimeValue::new(original)]);
    let roundtrip = call_meta("ast_to_string", &[parsed]);
    assert_eq!(roundtrip.as_str(), Some(original));
}

// ── MetaHost delegates to DefaultHost ───────────────────────

#[test]
fn meta_host_provides_default_functions() {
    let v = call_meta("add", &[RuntimeValue::new(1), RuntimeValue::new(2)]);
    assert_eq!(v.as_number(), Some(3.0));
}

#[test]
fn meta_host_provides_mul() {
    let v = call_meta("mul", &[RuntimeValue::new(3), RuntimeValue::new(4)]);
    assert_eq!(v.as_number(), Some(12.0));
}

#[test]
fn meta_host_provides_concat() {
    let v = call_meta(
        "concat",
        &[RuntimeValue::new("hello"), RuntimeValue::new(" world")],
    );
    assert_eq!(v.as_str(), Some("hello world"));
}

// ── helpers ─────────────────────────────────────────────────

fn call_meta(name: &str, args: &[RuntimeValue]) -> RuntimeValue {
    try_call_meta(name, args).expect("call should succeed")
}

fn try_call_meta(name: &str, args: &[RuntimeValue]) -> Result<RuntimeValue, ac_vm::Error> {
    let mut vm = Vm::new(MetaHost::default());
    let mut ctx = Context::new();
    let expr_args: Vec<Expr<RuntimeValue>> = args.iter().map(|a| Expr::value(a.clone())).collect();
    let expr = Expr::call(name, expr_args);
    vm.eval(&expr, &mut ctx)
}
