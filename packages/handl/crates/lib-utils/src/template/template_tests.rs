use handl_vm::Eval;
use handl_vm_backend_rust_interpreter::CanonicalEval;
use handl_vm_common::{Module, ModuleHost};
use handl_vm_values_json::{CanonicalContext, Error, FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

use super::TemplateModule;

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn object(pairs: &[(&str, JsonValue)]) -> NonFunctionValue {
    let map: serde_json::Map<String, JsonValue> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    NonFunctionValue::new(JsonValue::Object(map))
}

fn host() -> ModuleHost {
    ModuleHost::new()
        .with(TemplateModule)
        .with(crate::MathModule)
        .with(crate::StrModule)
        .with(crate::ObjModule)
        .with(crate::ArrayModule)
        .with(crate::FnModule)
}

fn ctx() -> CanonicalContext {
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(CanonicalEval::new(host(), None, None)));
    ctx
}

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    TemplateModule
        .function(name)
        .unwrap()
        .call(args, &mut ctx())
        .unwrap()
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Error {
    TemplateModule
        .function(name)
        .unwrap()
        .call(args, &mut ctx())
        .unwrap_err()
}

fn eval_expr(expr: &str, vars: &[(String, NonFunctionValue)]) -> NonFunctionValue {
    CanonicalEval::new(host(), None, None)
        .eval_expr(expr, vars)
        .unwrap()
}

// ── module metadata ────────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(TemplateModule.name(), "template");
}

#[test]
fn unknown_function_returns_none() {
    assert!(TemplateModule.function("nope").is_none());
}

// ── render: interpolation ──────────────────────────────────────────

#[test]
fn render_interpolates_variable() {
    let result = call(
        "render",
        &[
            s("Hello <%= name %>!"),
            object(&[("name", JsonValue::String("world".into()))]),
        ],
    );
    assert_eq!(result.as_str(), Some("Hello world!"));
}

#[test]
fn render_interpolates_function_result() {
    let result = call(
        "render",
        &[
            s("<%= upper(name) %>"),
            object(&[("name", JsonValue::String("world".into()))]),
        ],
    );
    assert_eq!(result.as_str(), Some("WORLD"));
}

#[test]
fn render_coerces_values() {
    let result = call(
        "render",
        &[
            s("<%= n %>:<%= flag %>:<%= empty %>"),
            object(&[
                ("n", JsonValue::from(42.0)),
                ("flag", JsonValue::Bool(true)),
                ("empty", JsonValue::Null),
            ]),
        ],
    );
    assert_eq!(result.as_str(), Some("42:true:null"));
}

#[test]
fn render_no_delimiters() {
    let result = call("render", &[s("just text"), object(&[])]);
    assert_eq!(result.as_str(), Some("just text"));
}

#[test]
fn render_empty_template() {
    let result = call("render", &[s(""), object(&[])]);
    assert_eq!(result.as_str(), Some(""));
}

#[test]
fn render_data_not_object() {
    let result = call("render", &[s("data=<%= data %>, it=<%= it %>"), n(42.0)]);
    assert_eq!(result.as_str(), Some("data=42, it=42"));
}

// ── render: escaping and silent evaluate ───────────────────────────

#[test]
fn render_escapes_html() {
    let result = call(
        "render",
        &[
            s("Value: <%- x %>"),
            object(&[("x", JsonValue::String("<script>".into()))]),
        ],
    );
    assert_eq!(result.as_str(), Some("Value: &lt;script&gt;"));
}

#[test]
fn render_evaluates_silently() {
    let result = call("render", &[s("A<% add(1,2) %>B"), object(&[])]);
    assert_eq!(result.as_str(), Some("AB"));
}

#[test]
fn render_mixed_delimiters() {
    let result = call(
        "render",
        &[
            s("Name: <%= name %>, Safe: <%- name %>, Eval: <% score %>Ok"),
            object(&[
                ("name", JsonValue::String("<b>".into())),
                ("score", JsonValue::from(10.0)),
            ]),
        ],
    );
    assert_eq!(
        result.as_str(),
        Some("Name: <b>, Safe: &lt;b&gt;, Eval: Ok")
    );
}

// ── render: options ────────────────────────────────────────────────

#[test]
fn render_custom_delimiters() {
    let opts = object(&[(
        "interpolate",
        JsonValue::Array(vec![
            JsonValue::String("{{".into()),
            JsonValue::String("}}".into()),
        ]),
    )]);
    let result = call(
        "render",
        &[
            s("Hello {{ name }}"),
            object(&[("name", JsonValue::String("world".into()))]),
            opts,
        ],
    );
    assert_eq!(result.as_str(), Some("Hello world"));
}

#[test]
fn render_variable_option() {
    let data = object(&[("name", JsonValue::String("world".into()))]);
    let opts = object(&[("variable", JsonValue::String("ctx".into()))]);
    let result = call(
        "render",
        &[s("Hello <%= obj.get(ctx, \"name\") %>"), data, opts],
    );
    assert_eq!(result.as_str(), Some("Hello world"));
}

#[test]
fn render_imports_option() {
    let imports = JsonValue::Object(serde_json::Map::from_iter([(
        "greeting".to_string(),
        JsonValue::String("hi".into()),
    )]));
    let opts = object(&[("imports", imports)]);
    let result = call("render", &[s("<%= greeting %>"), object(&[]), opts]);
    assert_eq!(result.as_str(), Some("hi"));
}

#[test]
fn render_null_options_ignored() {
    let result = call(
        "render",
        &[
            s("Hello <%= name %>"),
            object(&[("name", JsonValue::String("world".into()))]),
            NonFunctionValue::new(JsonValue::Null),
        ],
    );
    assert_eq!(result.as_str(), Some("Hello world"));
}

// ── render: errors ─────────────────────────────────────────────────

#[test]
fn render_missing_template_error() {
    let err = call_err("render", &[]);
    assert!(matches!(
        err,
        Error::ArgumentCount {
            expected: 2,
            got: 0
        }
    ));
}

#[test]
fn render_non_string_template_error() {
    let err = call_err("render", &[n(1.0), object(&[])]);
    assert!(matches!(
        err,
        Error::TypeMismatch {
            expected: "string",
            got: "non-string"
        }
    ));
}

#[test]
fn render_too_many_args_error() {
    let err = call_err("render", &[s(""), object(&[]), object(&[]), n(1.0)]);
    assert!(matches!(
        err,
        Error::ArgumentCount {
            expected: 3,
            got: 4
        }
    ));
}

#[test]
fn render_unclosed_delimiter_error() {
    let err = call_err("render", &[s("Hello <%= name"), object(&[])]);
    assert!(matches!(err, Error::PrimitiveError(_)));
}

#[test]
fn render_unknown_variable_error() {
    let err = call_err("render", &[s("<%= missing %>"), object(&[])]);
    assert!(matches!(err, Error::UnknownVariable(_)));
}

#[test]
fn render_invalid_options_error() {
    let err = call_err("render", &[s(""), object(&[]), n(1.0)]);
    assert!(matches!(
        err,
        Error::TypeMismatch {
            expected: "object",
            got: "non-object"
        }
    ));
}

#[test]
fn render_invalid_delimiter_error() {
    let opts = object(&[(
        "interpolate",
        JsonValue::Array(vec![JsonValue::String("{{".into())]),
    )]);
    let err = call_err("render", &[s(""), object(&[]), opts]);
    assert!(matches!(err, Error::PrimitiveError(_)));
}

// ── escape ─────────────────────────────────────────────────────────

#[test]
fn escape_html() {
    let result = call("escape", &[s("< > ' \" / &")]);
    assert_eq!(
        result.as_str(),
        Some("&lt; &gt; &#x27; &quot; &#x2F; &amp;")
    );
}

#[test]
fn escape_missing_arg_error() {
    let err = call_err("escape", &[]);
    assert!(matches!(
        err,
        Error::ArgumentCount {
            expected: 1,
            got: 0
        }
    ));
}

#[test]
fn escape_non_string_error() {
    let err = call_err("escape", &[n(1.0)]);
    assert!(matches!(
        err,
        Error::TypeMismatch {
            expected: "string",
            got: "non-string"
        }
    ));
}

// ── compile ────────────────────────────────────────────────────────

#[test]
fn compile_returns_function() {
    let result = call("compile", &[s("Hello <%= name %>")]);
    let fn_val = FunctionValue::from_json(result.inner()).unwrap();
    assert_eq!(fn_val.params.len(), 1);
    assert_eq!(fn_val.params[0].0, "data");
    assert_eq!(fn_val.body, r#"template.render("Hello <%= name %>", data)"#);
}

#[test]
fn compile_with_options_returns_function() {
    let opts = object(&[(
        "interpolate",
        JsonValue::Array(vec![
            JsonValue::String("{{".into()),
            JsonValue::String("}}".into()),
        ]),
    )]);
    let result = call("compile", &[s("Hello {{ name }}"), opts]);
    let fn_val = FunctionValue::from_json(result.inner()).unwrap();
    assert!(
        fn_val
            .body
            .starts_with(r#"template.render("Hello {{ name }}", data, {"#)
    );
    assert!(fn_val.body.contains("\"interpolate\""));
}

#[test]
fn compile_can_be_applied() {
    let compiled = call("compile", &[s("Hello <%= name %>")]);
    let data = object(&[("name", JsonValue::String("world".into()))]);
    let result = eval_expr(
        "fn.apply(compiled, [data])",
        &[
            ("compiled".to_string(), compiled),
            ("data".to_string(), data),
        ],
    );
    assert_eq!(result.as_str(), Some("Hello world"));
}

#[test]
fn compile_missing_template_error() {
    let err = call_err("compile", &[]);
    assert!(matches!(
        err,
        Error::ArgumentCount {
            expected: 1,
            got: 0
        }
    ));
}

#[test]
fn compile_too_many_args_error() {
    let err = call_err("compile", &[s(""), n(1.0), n(2.0)]);
    assert!(matches!(
        err,
        Error::ArgumentCount {
            expected: 2,
            got: 3
        }
    ));
}
