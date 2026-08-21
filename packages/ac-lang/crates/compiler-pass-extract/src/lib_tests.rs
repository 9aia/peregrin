use crate::ExtractPass;
use ac_compiler_core::{Compilation, Config, Pass, Source};
use ac_compiler_pass_parse::ParsePass;

fn test_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: None,
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
        extensions: Default::default(),
    }
}

fn parse_and_extract(yaml: &str) -> Compilation {
    let source = Source::new(yaml, "yaml");
    let mut compilation = Compilation::new(source);
    ParsePass.run(&mut compilation, &test_config()).unwrap();
    ExtractPass.run(&mut compilation, &test_config()).unwrap();
    compilation
}

#[test]
fn extract_pass_populates_extracted() {
    let compilation = parse_and_extract(
        r#"
$:
  name:
    type: string
  version:
    type: number

name: ac
version: 1
"#,
    );
    assert!(compilation.extracted.is_some());
    let extracted = compilation.extracted.unwrap();
    assert_eq!(extracted.schemas.len(), 2);
}

#[test]
fn extract_pass_no_fields_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    ParsePass.run(&mut compilation, &test_config()).unwrap();
    let result = ExtractPass.run(&mut compilation, &test_config());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no `$:` section"));
}

#[test]
fn extract_pass_without_parsed_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    let result = ExtractPass.run(&mut compilation, &test_config());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no parsed value"));
}

#[test]
fn extract_pass_takes_parsed_value() {
    let compilation = parse_and_extract(
        r#"
$:
  host:
    type: string

host: example.com
"#,
    );
    // parsed should be consumed (taken) after extract
    assert!(compilation.parsed.is_none());
    assert!(compilation.extracted.is_some());
}
