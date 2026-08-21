use ac_compiler_core::{Compilation, Config, Pass, Source};
use ac_compiler_pass_extract::ExtractPass;
use ac_compiler_pass_parse::ParsePass;
use ac_compiler_pass_resolve::ResolvePass;
use crate::GeneratePass;

fn test_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: Some(Box::new(
            ac_typegen_backend_json_schema::JsonSchemaBackend,
        )),
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
    }
}

fn full_pipeline(yaml: &str) -> Compilation {
    let source = Source::new(yaml, "yaml");
    let mut compilation = Compilation::new(source);
    let config = test_config();
    ParsePass.run(&mut compilation, &config).unwrap();
    ExtractPass.run(&mut compilation, &config).unwrap();
    ResolvePass.run(&mut compilation, &config).unwrap();
    GeneratePass.run(&mut compilation, &config).unwrap();
    compilation
}

#[test]
fn generate_pass_populates_output() {
    let compilation = full_pipeline(r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#);
    assert!(compilation.output.is_some());
    let output = compilation.output.unwrap();
    assert_eq!(output["type"], "object");
    assert!(output["properties"]["host"]["type"].is_string());
}

#[test]
fn generate_pass_without_ctx_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    let config = test_config();
    let result = GeneratePass.run(&mut compilation, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no typegen context"));
}

#[test]
fn generate_pass_without_backend_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    let config_no_backend = Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: None,
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
    };
    // Need to manually set typegen_ctx since we can't run ResolvePass without the right deps
    compilation.typegen_ctx = Some(ac_typegen_core::TypegenContext::from_schemas(&[]));
    let result = GeneratePass.run(&mut compilation, &config_no_backend);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no backend configured"));
}

#[test]
fn generate_pass_takes_ctx() {
    let compilation = full_pipeline(r#"
$:
  host:
    type: string

host: example.com
"#);
    // ctx should be consumed after generate
    assert!(compilation.typegen_ctx.is_none());
    assert!(compilation.output.is_some());
}
