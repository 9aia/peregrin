use ac_engine_core::{Compilation, Config, Pass, Source};
use ac_engine_pass_extract::ExtractPass;
use ac_engine_pass_parse::ParsePass;
use crate::ResolvePass;

fn test_config() -> Config {
    Config {
        resolver: Box::new(ac_resolver_virtual::VirtualResolver::new()),
        interpreter: Box::new(ac_typecheck_schema_json::JsonSchemaInterpreter),
        backend: Some(Box::new(
            ac_typegen_backend_json_schema::JsonSchemaBackend,
        )),
    }
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
    let compilation = parse_extract_resolve(r#"
fields:
  host:
    type: string
  port:
    type: number

host: example.com
port: 8080
"#);
    assert!(compilation.typegen_ctx.is_some());
}

#[test]
fn resolve_pass_without_extracted_returns_error() {
    let source = Source::new("name: ac", "yaml");
    let mut compilation = Compilation::new(source);
    let config = test_config();
    let result = ResolvePass.run(&mut compilation, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no extracted fields"));
}

#[test]
fn resolve_pass_strips_ac_keys() {
    let compilation = parse_extract_resolve(r#"
fields:
  host:
    type: string
    mutable: true
  port:
    type: number
    expr: "8080"

host: example.com
port: 8080
"#);
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
    let compilation = parse_extract_resolve(r#"
fields:
  _base:
    type: object
    merge: true
  host:
    type: string

_base:
  host: example.com
host: override.com
"#);
    assert!(compilation.typegen_ctx.is_some());
    let ctx = compilation.typegen_ctx.unwrap();
    // _base should be filtered out from schemas
    let base = ctx.fields.iter().find(|(n, _)| n.as_str() == "_base");
    assert!(base.is_none());
}
