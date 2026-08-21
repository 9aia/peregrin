use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::StrModule;

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = StrModule.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = StrModule.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

// ── capitalize ──────────────────────────────────────────────────────

#[test]
fn capitalize_basic() {
    assert_eq!(call("capitalize", &[s("hello")]).as_str(), Some("Hello"));
}

#[test]
fn capitalize_already_capitalized() {
    assert_eq!(call("capitalize", &[s("Hello")]).as_str(), Some("Hello"));
}

#[test]
fn capitalize_uppercase_rest() {
    assert_eq!(call("capitalize", &[s("hELLO")]).as_str(), Some("Hello"));
}

#[test]
fn capitalize_empty_string() {
    assert_eq!(call("capitalize", &[s("")]).as_str(), Some(""));
}

#[test]
fn capitalize_unicode() {
    assert_eq!(call("capitalize", &[s("café")]).as_str(), Some("Café"));
}

#[test]
fn capitalize_no_args_errors() {
    let result = call_err("capitalize", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn capitalize_non_string_errors() {
    let result = call_err("capitalize", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── title_case ──────────────────────────────────────────────────────

#[test]
fn title_case_basic() {
    assert_eq!(
        call("title_case", &[s("hello world")]).as_str(),
        Some("Hello World")
    );
}

#[test]
fn title_case_already_title() {
    assert_eq!(
        call("title_case", &[s("Hello World")]).as_str(),
        Some("Hello World")
    );
}

#[test]
fn title_case_mixed_case() {
    assert_eq!(
        call("title_case", &[s("hELLO wORLD")]).as_str(),
        Some("Hello World")
    );
}

#[test]
fn title_case_single_word() {
    assert_eq!(call("title_case", &[s("hello")]).as_str(), Some("Hello"));
}

#[test]
fn title_case_empty_string() {
    assert_eq!(call("title_case", &[s("")]).as_str(), Some(""));
}

#[test]
fn title_case_multiple_spaces() {
    assert_eq!(
        call("title_case", &[s("hello  world")]).as_str(),
        Some("Hello  World")
    );
}

#[test]
fn title_case_tab_boundary() {
    assert_eq!(
        call("title_case", &[s("hello\tworld")]).as_str(),
        Some("Hello\tWorld")
    );
}

#[test]
fn title_case_newline_boundary() {
    assert_eq!(
        call("title_case", &[s("hello\nworld")]).as_str(),
        Some("Hello\nWorld")
    );
}

#[test]
fn title_case_no_args_errors() {
    let result = call_err("title_case", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn title_case_non_string_errors() {
    let result = call_err("title_case", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── snake_case ──────────────────────────────────────────────────────

#[test]
fn snake_case_from_camel() {
    assert_eq!(
        call("snake_case", &[s("helloWorld")]).as_str(),
        Some("hello_world")
    );
}

#[test]
fn snake_case_from_pascal() {
    assert_eq!(
        call("snake_case", &[s("HelloWorld")]).as_str(),
        Some("hello_world")
    );
}

#[test]
fn snake_case_from_kebab() {
    assert_eq!(
        call("snake_case", &[s("hello-world")]).as_str(),
        Some("hello_world")
    );
}

#[test]
fn snake_case_from_spaces() {
    assert_eq!(
        call("snake_case", &[s("hello world")]).as_str(),
        Some("hello_world")
    );
}

#[test]
fn snake_case_already_snake() {
    assert_eq!(
        call("snake_case", &[s("hello_world")]).as_str(),
        Some("hello_world")
    );
}

#[test]
fn snake_case_single_word() {
    assert_eq!(call("snake_case", &[s("hello")]).as_str(), Some("hello"));
}

#[test]
fn snake_case_empty_string() {
    assert_eq!(call("snake_case", &[s("")]).as_str(), Some(""));
}

#[test]
fn snake_case_consecutive_uppercase() {
    assert_eq!(
        call("snake_case", &[s("parseHTMLString")]).as_str(),
        Some("parse_html_string")
    );
}

#[test]
fn snake_case_no_args_errors() {
    let result = call_err("snake_case", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn snake_case_non_string_errors() {
    let result = call_err("snake_case", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// regression: multi-byte uppercase
#[test]
fn snake_case_multibyte_uppercase() {
    assert_eq!(
        call("snake_case", &[s("ÉlloWorld")]).as_str(),
        Some("éllo_world")
    );
}

// ── kebab_case ──────────────────────────────────────────────────────

#[test]
fn kebab_case_from_camel() {
    assert_eq!(
        call("kebab_case", &[s("helloWorld")]).as_str(),
        Some("hello-world")
    );
}

#[test]
fn kebab_case_from_pascal() {
    assert_eq!(
        call("kebab_case", &[s("HelloWorld")]).as_str(),
        Some("hello-world")
    );
}

#[test]
fn kebab_case_from_snake() {
    assert_eq!(
        call("kebab_case", &[s("hello_world")]).as_str(),
        Some("hello-world")
    );
}

#[test]
fn kebab_case_from_spaces() {
    assert_eq!(
        call("kebab_case", &[s("hello world")]).as_str(),
        Some("hello-world")
    );
}

#[test]
fn kebab_case_single_word() {
    assert_eq!(call("kebab_case", &[s("hello")]).as_str(), Some("hello"));
}

#[test]
fn kebab_case_empty_string() {
    assert_eq!(call("kebab_case", &[s("")]).as_str(), Some(""));
}

#[test]
fn kebab_case_no_args_errors() {
    let result = call_err("kebab_case", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn kebab_case_non_string_errors() {
    let result = call_err("kebab_case", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// regression: multi-byte uppercase
#[test]
fn kebab_case_multibyte_uppercase() {
    assert_eq!(
        call("kebab_case", &[s("ÉlloWorld")]).as_str(),
        Some("éllo-world")
    );
}

// ── camel_case ──────────────────────────────────────────────────────

#[test]
fn camel_case_from_snake() {
    assert_eq!(
        call("camel_case", &[s("hello_world")]).as_str(),
        Some("helloWorld")
    );
}

#[test]
fn camel_case_from_pascal() {
    assert_eq!(
        call("camel_case", &[s("HelloWorld")]).as_str(),
        Some("helloWorld")
    );
}

#[test]
fn camel_case_from_kebab() {
    assert_eq!(
        call("camel_case", &[s("hello-world")]).as_str(),
        Some("helloWorld")
    );
}

#[test]
fn camel_case_from_spaces() {
    assert_eq!(
        call("camel_case", &[s("hello world")]).as_str(),
        Some("helloWorld")
    );
}

#[test]
fn camel_case_single_word() {
    assert_eq!(call("camel_case", &[s("hello")]).as_str(), Some("hello"));
}

#[test]
fn camel_case_empty_string() {
    assert_eq!(call("camel_case", &[s("")]).as_str(), Some(""));
}

#[test]
fn camel_case_no_args_errors() {
    let result = call_err("camel_case", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn camel_case_non_string_errors() {
    let result = call_err("camel_case", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// regression: multi-byte uppercase
#[test]
fn camel_case_multibyte_uppercase() {
    assert_eq!(
        call("camel_case", &[s("Éllo World")]).as_str(),
        Some("élloWorld")
    );
}

// ── pascal_case ─────────────────────────────────────────────────────

#[test]
fn pascal_case_from_snake() {
    assert_eq!(
        call("pascal_case", &[s("hello_world")]).as_str(),
        Some("HelloWorld")
    );
}

#[test]
fn pascal_case_from_camel() {
    assert_eq!(
        call("pascal_case", &[s("helloWorld")]).as_str(),
        Some("HelloWorld")
    );
}

#[test]
fn pascal_case_from_kebab() {
    assert_eq!(
        call("pascal_case", &[s("hello-world")]).as_str(),
        Some("HelloWorld")
    );
}

#[test]
fn pascal_case_from_spaces() {
    assert_eq!(
        call("pascal_case", &[s("hello world")]).as_str(),
        Some("HelloWorld")
    );
}

#[test]
fn pascal_case_single_word() {
    assert_eq!(call("pascal_case", &[s("hello")]).as_str(), Some("Hello"));
}

#[test]
fn pascal_case_empty_string() {
    assert_eq!(call("pascal_case", &[s("")]).as_str(), Some(""));
}

#[test]
fn pascal_case_no_args_errors() {
    let result = call_err("pascal_case", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pascal_case_non_string_errors() {
    let result = call_err("pascal_case", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// regression: multi-byte uppercase
#[test]
fn pascal_case_multibyte_uppercase() {
    assert_eq!(
        call("pascal_case", &[s("Éllo World")]).as_str(),
        Some("ÉlloWorld")
    );
}
