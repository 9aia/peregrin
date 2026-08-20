//! Integration tests for the low-level fields pipeline.
//!
//! These test the extract → interpolate → compute → merge → validate
//! flow directly, without going through the engine pipeline.

use ac_fields::{
    self, extract_fields, interpolate, compute_fields, merge_fields,
    build_doc_namespace, FunctionResolver, Namespaces,
};
use ac_dialect_canonical::AC_DIALECT;
use ac_frontend_yaml;
use ac_runtime::RuntimeValue;
use ac_schema::Schema;
use ac_schema_json::JsonSchema;
use serde_json::Value as JsonValue;

struct AddResolver;
impl FunctionResolver for AddResolver {
    fn call(&self, name: &str, args: &[JsonValue]) -> Result<JsonValue, String> {
        match name {
            "add" => {
                let sum: f64 = args.iter().filter_map(|v| v.as_f64()).sum();
                serde_json::Number::from_f64(sum)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            "mul" => {
                let product: f64 = args.iter().filter_map(|v| v.as_f64()).product();
                serde_json::Number::from_f64(product)
                    .map(JsonValue::Number)
                    .ok_or("NaN".to_string())
            }
            _ => Err(format!("unknown function: {name}")),
        }
    }
}

#[test]
fn full_pipeline_parse_extract_interpolate_compute_validate() {
    let yaml = r#"
fields:
  name:
    type: string
  version:
    type: number
  doubled:
    type: number
    expr: "mul(version, 2)"

name: ac
version: 3
"#;

    let value = ac_frontend_yaml::parse(yaml).unwrap();
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();

    assert_eq!(extracted.schemas.len(), 3);
    assert_eq!(extracted.computed.len(), 1);

    let mut namespaces = Namespaces::new();
    namespaces.insert("doc".to_string(), build_doc_namespace(&extracted.data));

    let interpolated = interpolate(extracted.data.inner(), &namespaces, &AddResolver, &AC_DIALECT);
    let mut data = RuntimeValue::new(interpolated);

    let compute_errors = compute_fields(data.inner_mut(), &extracted.computed, &mut namespaces, &AddResolver, &AC_DIALECT);
    assert!(compute_errors.is_empty());

    assert_eq!(data.get("name").unwrap().as_str(), Some("ac"));
    assert_eq!(data.get("version").unwrap().as_i64(), Some(3));
    assert_eq!(data.get("doubled").unwrap().as_f64(), Some(6.0));

    for (name, raw) in &extracted.schemas {
        let schema = JsonSchema::new(raw.clone());
        let field_value = data.get(name).unwrap_or(&JsonValue::Null);
        let field_rv = RuntimeValue::new(field_value.clone());
        assert!(schema.validate(&field_rv).is_ok(), "field {name} should validate");
    }
}

#[test]
fn full_pipeline_with_merge() {
    let yaml = r#"
fields:
  _base:
    type: object
    merge: true
  host:
    type: string
  port:
    type: number

_base:
  host: example.com
  port: 8080
host: override.com
"#;

    let value = ac_frontend_yaml::parse(yaml).unwrap();
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();

    assert_eq!(extracted.merge, vec!["_base"]);

    let mut data = extracted.data.into_inner();

    let merge_errors = merge_fields(&mut data, &extracted.merge);
    assert!(merge_errors.is_empty());

    assert_eq!(data["host"], "override.com");
    assert_eq!(data["port"], 8080);
    assert!(data.as_object().unwrap().get("_base").is_none());
}

#[test]
fn full_pipeline_yaml_to_json_to_toml_roundtrip() {
    let yaml = "name: ac\nversion: 1\n";
    let value = ac_frontend_yaml::parse(yaml).unwrap();

    let json_str = ac_frontend_json::to_string_pretty(&value).unwrap();
    let value_from_json = ac_frontend_json::parse(&json_str).unwrap();

    let toml_str = ac_frontend_toml::to_string(&value_from_json).unwrap();
    let value_from_toml = ac_frontend_toml::parse(&toml_str).unwrap();

    assert_eq!(value_from_toml.get("name").unwrap().as_str(), Some("ac"));
    assert_eq!(value_from_toml.get("version").unwrap().as_i64(), Some(1));
}

#[test]
fn full_pipeline_undeclared_detection() {
    let yaml = r#"
fields:
  name:
    type: string

name: ac
extra: true
"#;

    let value = ac_frontend_yaml::parse(yaml).unwrap();
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();

    let declared: Vec<&str> = extracted.schemas.iter().map(|(n, _)| n.as_str()).collect();
    let undeclared = ac_fields::undeclared_fields(&extracted.data, &declared);
    assert_eq!(undeclared, vec!["extra"]);
}
