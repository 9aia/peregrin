use crate::ParsePass;
use ac_compiler_core::{Compilation, Config, Pass, Source};

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

#[test]
fn parse_pass_yaml_populates_parsed() {
    let source = Source::new("name: ac\nversion: 1", "yaml");
    let mut compilation = Compilation::new(source);
    ParsePass.run(&mut compilation, &test_config()).unwrap();

    assert!(compilation.parsed.is_some());
    let parsed = compilation.parsed.unwrap();
    assert_eq!(parsed.get("name").unwrap().as_str(), Some("ac"));
    assert_eq!(parsed.get("version").unwrap().as_i64(), Some(1));
}

#[test]
fn parse_pass_json_populates_parsed() {
    let source = Source::new(r#"{"name": "ac", "version": 1}"#, "json");
    let mut compilation = Compilation::new(source);
    ParsePass.run(&mut compilation, &test_config()).unwrap();

    assert!(compilation.parsed.is_some());
    let parsed = compilation.parsed.unwrap();
    assert_eq!(parsed.get("name").unwrap().as_str(), Some("ac"));
}

#[test]
fn parse_pass_invalid_format_returns_error() {
    let source = Source::new("name: ac", "xml");
    let mut compilation = Compilation::new(source);
    let result = ParsePass.run(&mut compilation, &test_config());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown format"));
}

#[test]
fn parse_pass_invalid_yaml_returns_error() {
    let source = Source::new("name: ac\n  bad: : :", "yaml");
    let mut compilation = Compilation::new(source);
    let result = ParsePass.run(&mut compilation, &test_config());
    assert!(result.is_err());
}

#[test]
fn parse_pass_toml_populates_parsed() {
    let source = Source::new("name = \"ac\"\nversion = 1", "toml");
    let mut compilation = Compilation::new(source);
    ParsePass.run(&mut compilation, &test_config()).unwrap();

    assert!(compilation.parsed.is_some());
    let parsed = compilation.parsed.unwrap();
    assert_eq!(parsed.get("name").unwrap().as_str(), Some("ac"));
    assert_eq!(parsed.get("version").unwrap().as_i64(), Some(1));
}
