use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::module::IntlModule;

fn call(args: &[NonFunctionValue]) -> NonFunctionValue {
    let module = IntlModule::new();
    let func = module.function("segment").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let module = IntlModule::new();
    let func = module.function("segment").expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn obj(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

fn get_segments(result: &NonFunctionValue) -> Vec<String> {
    let arr = result.as_array().expect("array result");
    arr.iter().map(|v| {
        if let Some(obj) = v.as_object() {
            obj.get("segment").and_then(|s| s.as_str()).expect("segment field").to_string()
        } else {
            v.as_str().expect("string element").to_string()
        }
    }).collect()
}

#[test]
fn segment_word_hello_world() {
    let result = call(&[s("Hello World"), s("word")]);
    let arr = result.as_array().expect("array result");
    let first = arr.first().expect("at least one segment");
    let obj = first.as_object().expect("segment object");
    assert!(obj.contains_key("segment"));
    assert!(obj.contains_key("index"));
    assert!(obj.contains_key("isWordLike"));
    let segments = get_segments(&result);
    assert!(segments.contains(&"Hello".to_string()));
    assert!(segments.contains(&"World".to_string()));
}

#[test]
fn segment_grapheme_hello() {
    let result = call(&[s("Hello"), s("grapheme")]);
    let segments = get_segments(&result);
    assert_eq!(segments.len(), 5);
    assert_eq!(segments[0], "H");
    assert_eq!(segments[4], "o");
}

#[test]
fn segment_sentence() {
    let result = call(&[s("Hello. World."), s("sentence")]);
    let segments = get_segments(&result);
    assert!(segments.len() >= 2);
}

#[test]
fn segment_word_empty() {
    let result = call(&[s(""), s("word")]);
    let segments = get_segments(&result);
    assert!(segments.is_empty());
}

#[test]
fn segment_grapheme_empty() {
    let result = call(&[s(""), s("grapheme")]);
    let segments = get_segments(&result);
    assert!(segments.is_empty());
}

#[test]
fn segment_no_args_errors() {
    assert!(call_err(&[]).is_err());
}

#[test]
fn segment_missing_granularity_errors() {
    assert!(call_err(&[s("Hello")]).is_err());
}

#[test]
fn segment_invalid_granularity_errors() {
    assert!(call_err(&[s("Hello"), s("paragraph")]).is_err());
}

#[test]
fn segment_word_multiple_spaces() {
    let result = call(&[s("a b c"), s("word")]);
    let segments = get_segments(&result);
    assert!(segments.contains(&"a".to_string()));
    assert!(segments.contains(&"b".to_string()));
    assert!(segments.contains(&"c".to_string()));
}

#[test]
fn segment_grapheme_unicode() {
    let result = call(&[s("café"), s("grapheme")]);
    let segments = get_segments(&result);
    assert_eq!(segments.len(), 4);
    assert_eq!(segments[3], "é");
}

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::Vm;

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(IntlModule::new());
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

#[test]
fn vm_segment() {
    let expr = Expr::call("intl.segment", vec![Expr::value(s("Hello World")), Expr::value(s("word"))]);
    let result = eval_expr(&expr);
    assert!(result.as_array().is_some());
}

#[test]
fn segment_line() {
    let result = call(&[s("Hello World Foo Bar"), s("line")]);
    let arr = result.as_array().expect("array result");
    assert!(!arr.is_empty());
}

#[test]
fn segment_line_break_style_strict() {
    let result = call(&[s("Hello World"), s("line"), s("ja"), obj(&[("lineBreakStyle", "strict")])]);
    let arr = result.as_array().expect("array result");
    assert!(!arr.is_empty());
}

#[test]
fn segment_line_break_style_loose() {
    let result = call(&[s("Hello World"), s("line"), s("ja"), obj(&[("lineBreakStyle", "loose")])]);
    let arr = result.as_array().expect("array result");
    assert!(!arr.is_empty());
}

#[test]
fn segment_line_break_style_normal() {
    let result = call(&[s("Hello World"), s("line"), s("ja"), obj(&[("lineBreakStyle", "normal")])]);
    let arr = result.as_array().expect("array result");
    assert!(!arr.is_empty());
}

#[test]
fn segment_line_uses_ja_locale() {
    let ja_strict = call(&[s("おっー"), s("line"), s("ja"), obj(&[("lineBreakStyle", "strict")])]);
    let ja_normal = call(&[s("おっー"), s("line"), s("ja"), obj(&[("lineBreakStyle", "normal")])]);
    let ja_segments = get_segments(&ja_normal);
    let ja_strict_segments = get_segments(&ja_strict);
    assert!(
        ja_segments.len() >= ja_strict_segments.len(),
        "ja locale with normal strictness should produce at least as many segments as strict. \
        normal: {:?}, strict: {:?}",
        ja_segments,
        ja_strict_segments
    );
}
