use crate::ResolvePass;
use ac_compiler_core::{Compilation, Config, Pass, Source};
use ac_compiler_pass_extract::ExtractPass;
use ac_compiler_pass_parse::ParsePass;

fn test_config() -> Config {
    Config {
        dialect: Box::new(ac_dialect_canonical::AC_DIALECT),
        resolver: Box::new(ac_module_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: Some(Box::new(ac_typegen_backend_json_schema::JsonSchemaBackend)),
        key_handler: Box::new(ac_typegen_key_set_special::SpecialKeyHandler),
        import_resolver: Box::new(ac_typegen_import::DefaultImportResolver),
        extensions: Default::default()}
}

fn parse_extract_resolve(yaml: &str) -> Compilation {
    let source = Source::new(yaml, "yaml");
    let mut compilation = Compilation::new(source);
    let config = test_config();
    ParsePass.run(&mut compilation, &config).unwrap();
    ExtractPass.run(&mut compilation, &config).unwrap();
    ResolvePass.run(&mut compilation, &config).unwrap();
    compilation
}

#[test]
fn resolve_pass_populates_typegen_ctx() {
    let compilation = parse_extract_resolve(
        r#"
$:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#,
    );
    assert!(compilation.typegen_ctx.is_some());
}

#[test]
fn resolve_pass_without_extracted_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    let config = test_config();
    let result = ResolvePass.run(&mut compilation, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no extracted fields"));
}

#[test]
fn resolve_pass_strips_ac_keys() {
    let compilation = parse_extract_resolve(
        r#"
$:
  host:
    type: string
    mutable: true
  port:
    type: number
    expr: "8080"

host: example.com
port: 8080
"#,
    );
    assert!(compilation.typegen_ctx.is_some());
    let ctx = compilation.typegen_ctx.unwrap();
    // After stripping, the schemas should not contain AC keys
    let host_schema = ctx.fields.iter().find(|(n, _)| n.as_str() == "host");
    assert!(host_schema.is_some());
    let (_, host) = host_schema.unwrap();
    assert!(host.get("mutable").is_none());
}

#[test]
fn resolve_pass_filters_merge_fields() {
    let compilation = parse_extract_resolve(
        r#"
$:
  _base:
    type: object
    merge: true
  host:
    type: string

_base:
  host: example.com
host: override.com
"#,
    );
    assert!(compilation.typegen_ctx.is_some());
    let ctx = compilation.typegen_ctx.unwrap();
    // _base should be filtered out from schemas
    let base = ctx.fields.iter().find(|(n, _)| n.as_str() == "_base");
    assert!(base.is_none());
}
