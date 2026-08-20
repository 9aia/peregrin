//! Self-describing documents: inline `fields:` schemas + `${doc.*}` interpolation.
//!
//! Demonstrates a file that declares its own field schemas, uses
//! `${doc.name}` to reference other fields, and validates
//! everything against its declared schemas.
//!
//! Run: `cargo run --example fields`

use std::path::PathBuf;

use ac_dialect_canonical::AC_DIALECT;
use ac_runtime::RuntimeValue;
use ac_schema::Schema;
use ac_schema_json::JsonSchema;

fn main() {
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fields/input.yml");
    let yaml_input = std::fs::read_to_string(&yaml_path)
        .unwrap_or_else(|_| panic!("failed to read {}", yaml_path.display()));

    let value = ac_frontend_yaml::parse(&yaml_input).unwrap();
    println!("parsed from {}:", yaml_path.display());
    println!("{}", ac_frontend_yaml::to_string(&value).unwrap());

    let extracted = ac_fields::extract_fields(value, &AC_DIALECT).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    if extracted.schemas.is_empty() {
        eprintln!("error: no `fields:` section found in input");
        std::process::exit(1);
    }

    let field_schemas = extracted.schemas;
    let data = extracted.data;

    // Construct schema objects from raw JSON values
    let schemas: Vec<(String, JsonSchema)> = field_schemas
        .iter()
        .map(|(name, raw)| (name.clone(), JsonSchema::new(raw.clone())))
        .collect();

    println!("declared fields:");
    for (name, schema) in &schemas {
        let type_name = schema.type_name().unwrap_or("any");
        println!("  {name}: {type_name}");
    }
    println!();

    // Build namespaces for interpolation
    let mut namespaces = ac_fields::Namespaces::new();
    namespaces.insert("doc".to_string(), ac_fields::build_doc_namespace(&data));
    namespaces.insert("env".to_string(), ac_fields::build_env_namespace());

    // Interpolate ${doc.*} and ${env.*} in all data values
    let interpolated = ac_fields::interpolate(data.inner(), &namespaces, &ac_fields::NoFunctions, &ac_dialect_canonical::AC_DIALECT);
    let data = RuntimeValue::new(interpolated);

    println!("interpolated values:");
    for (name, _) in &schemas {
        let val = data.get(name).unwrap_or(&serde_json::Value::Null);
        println!("  {name}: {val}");
    }
    println!();

    // Validate interpolated values
    let mut errors = 0;
    for (name, schema) in &schemas {
        let field_value = data.get(name).unwrap_or(&serde_json::Value::Null);
        let field_rv = RuntimeValue::new(field_value.clone());
        match schema.validate(&field_rv) {
            Ok(()) => println!("  {name}: ok"),
            Err(e) => {
                eprintln!("  {name}: INVALID — {e}");
                errors += 1;
            }
        }
    }

    // Check for undeclared fields
    let declared: Vec<&str> = schemas.iter().map(|(n, _)| n.as_str()).collect();
    for key in ac_fields::undeclared_fields(&data, &declared) {
        eprintln!("  {key}: UNDECLARED — not in fields:");
        errors += 1;
    }

    if errors == 0 {
        println!("\nall fields valid");
    } else {
        eprintln!("\n{errors} validation error(s)");
        std::process::exit(1);
    }
}