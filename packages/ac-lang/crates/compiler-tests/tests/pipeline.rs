//! Integration tests for the AC engine pipeline.
//!
//! These tests exercise the full pipeline end-to-end using the built-in
//! passes and preset configurations from `ac-compiler`.

use ac_compiler::{Compilation, Config, Pass, Pipeline, Source};
use ac_compiler_pass_extract::ExtractPass;
use ac_compiler_pass_parse::ParsePass;

fn default_config() -> Config {
    ac_compiler::default_cli_config()
}

fn typecheck_config() -> Config {
    ac_compiler::typecheck_only_config()
}

// --- Typegen pipeline preset ---

#[test]
fn typegen_pipeline_generates_json_schema() {
    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = ac_compiler::typegen_pipeline(default_config());
    let compilation = pipeline.run(source).unwrap();

    let output = compilation.output.expect("output should be generated");
    assert_eq!(output["type"], "object");
    assert_eq!(output["properties"]["host"]["type"], "string");
    assert_eq!(output["properties"]["port"]["type"], "number");
}

#[test]
fn typegen_pipeline_with_merge_fields() {
    let yaml = r#"
$:
  _base:
    type: object
    merge: true
  host:
    type: string

_base:
  host: example.com
host: override.com
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = ac_compiler::typegen_pipeline(default_config());
    let compilation = pipeline.run(source).unwrap();

    let output = compilation.output.expect("output should be generated");
    assert_eq!(output["properties"]["host"]["type"], "string");
    // _base should be filtered out
    assert!(output["properties"].get("_base").is_none());
}

#[test]
fn typegen_pipeline_with_ac_keys_stripped() {
    let yaml = r#"
$:
  host:
    type: string
    mutable: true
    expr: "example.com"

host: example.com
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = ac_compiler::typegen_pipeline(default_config());
    let compilation = pipeline.run(source).unwrap();

    let output = compilation.output.expect("output should be generated");
    // AC keys should be stripped from the output schema
    assert!(output["properties"]["host"].get("mutable").is_none());
    assert!(output["properties"]["host"].get("expr").is_none());
}

#[test]
fn typegen_pipeline_invalid_format_returns_error() {
    let source = Source::new("name: ac", "xml");
    let pipeline = ac_compiler::typegen_pipeline(default_config());
    let result = pipeline.run(source);
    match &result {
        Err(e) => assert!(e.contains("unknown format")),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn typegen_pipeline_no_fields_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let pipeline = ac_compiler::typegen_pipeline(default_config());
    let result = pipeline.run(source);
    match &result {
        Err(e) => assert!(e.contains("no `$:` section")),
        Ok(_) => panic!("expected error"),
    }
}

// --- Typecheck pipeline preset ---

#[test]
fn typecheck_pipeline_valid_fields_no_errors() {
    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = ac_compiler::typecheck_pipeline(typecheck_config());
    let compilation = pipeline.run(source).unwrap();

    assert!(compilation.type_errors.is_empty());
}

#[test]
fn typecheck_pipeline_merge_non_object_reports_error() {
    let yaml = r#"
$:
  _base:
    type: string
    merge: true
  host:
    type: string

_base: hello
host: example.com
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = ac_compiler::typecheck_pipeline(typecheck_config());
    let compilation = pipeline.run(source).unwrap();

    assert!(!compilation.type_errors.is_empty());
}

// --- Full pipeline preset ---

#[test]
fn full_pipeline_typecheck_then_generate() {
    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = ac_compiler::full_pipeline(default_config());
    let compilation = pipeline.run(source).unwrap();

    assert!(compilation.type_errors.is_empty());
    assert!(compilation.output.is_some());
}

// --- Custom pipeline composition ---

#[test]
fn custom_pipeline_parse_extract_only() {
    let yaml = r#"
$:
  host:
    type: string

host: example.com
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = Pipeline::new(default_config())
        .with_pass(ParsePass)
        .with_pass(ExtractPass);

    let compilation = pipeline.run(source).unwrap();
    assert!(compilation.parsed.is_none()); // taken by ExtractPass
    assert!(compilation.extracted.is_some());
    assert!(compilation.typegen_ctx.is_none());
    assert!(compilation.output.is_none());
}

#[test]
fn custom_pipeline_with_custom_pass() {
    use serde_json::Value as JsonValue;

    struct CountFieldsPass;
    impl Pass for CountFieldsPass {
        fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
            if let Some(extracted) = &compilation.extracted {
                compilation.output = Some(JsonValue::Number(extracted.schemas.len().into()));
            }
            Ok(())
        }
    }

    let yaml = r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#;
    let source = Source::new(yaml, "yaml");
    let pipeline = Pipeline::new(default_config())
        .with_pass(ParsePass)
        .with_pass(ExtractPass)
        .with_pass(CountFieldsPass);

    let compilation = pipeline.run(source).unwrap();
    assert_eq!(compilation.output, Some(JsonValue::Number(2.into())));
}

// --- Pipeline ordering ---

#[test]
fn pipeline_passes_run_in_declaration_order() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ORDER: AtomicUsize = AtomicUsize::new(0);

    struct FirstPass;
    impl Pass for FirstPass {
        fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
            let n = ORDER.fetch_add(1, Ordering::SeqCst);
            compilation
                .type_errors
                .push(ac_typecheck_core::TypeError::custom(
                    "order",
                    format!("first:{n}"),
                    "",
                ));
            Ok(())
        }
    }

    struct SecondPass;
    impl Pass for SecondPass {
        fn run(&self, compilation: &mut Compilation, _config: &Config) -> Result<(), String> {
            let n = ORDER.fetch_add(1, Ordering::SeqCst);
            compilation
                .type_errors
                .push(ac_typecheck_core::TypeError::custom(
                    "order",
                    format!("second:{n}"),
                    "",
                ));
            Ok(())
        }
    }

    ORDER.store(0, Ordering::SeqCst);

    let source = Source::new("name: ac", "yaml");
    let pipeline = Pipeline::new(default_config())
        .with_pass(FirstPass)
        .with_pass(SecondPass);

    let compilation = pipeline.run(source).unwrap();
    assert_eq!(compilation.type_errors.len(), 2);
    assert!(compilation.type_errors[0].message.contains("first:0"));
    assert!(compilation.type_errors[1].message.contains("second:1"));
}

// --- Source with path ---

#[test]
fn pipeline_source_with_path_preserves_base_path() {
    let source = Source::new("name: ac", "yaml").with_path("/tmp/dir/test.yaml");
    let pipeline = Pipeline::new(default_config()).with_pass(ParsePass);
    let compilation = pipeline.run(source).unwrap();

    assert_eq!(
        compilation.source.base_path(),
        Some(std::path::PathBuf::from("/tmp/dir"))
    );
}
