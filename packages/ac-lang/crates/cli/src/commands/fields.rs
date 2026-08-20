use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use ac_fields;
use ac_runtime::RuntimeValue;
use ac_schema::Schema;
use ac_schema_json::JsonSchema;

use crate::cli::FieldsArgs;
use crate::frontend;
use crate::host::DefaultHost;
use crate::io;
use crate::resolver::VmResolver;

pub fn run(args: FieldsArgs) -> i32 {
    let input = match io::read_input(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let frontend = match frontend::frontend_for(&args.format) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let value = match frontend.parse(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let extracted = match ac_fields::extract_fields(value) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    if extracted.schemas.is_empty() {
        eprintln!("error: no `fields:` section found in input");
        return 1;
    }

    let field_schemas = extracted.schemas;
    let function_sigs = extracted.functions;
    let inheritable = extracted.inheritable;
    let immutable = extracted.immutable;
    let computed = extracted.computed;
    let merge = extracted.merge;
    let data = extracted.data;

    let mut namespaces = ac_fields::Namespaces::new();
    namespaces.insert("doc".to_string(), ac_fields::build_doc_namespace(&data));
    namespaces.insert("env".to_string(), ac_fields::build_env_namespace());

    let host = DefaultHost::default();
    let base_path = args.input.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let resolver = VmResolver {
        host,
        function_sigs: &function_sigs,
        namespaces: namespaces.clone(),
        depth: Cell::new(0),
        base_path,
        import_cache: RefCell::new(HashMap::new()),
        imported_functions: RefCell::new(Vec::new()),
    };

    let interpolated = ac_fields::interpolate_scoped(
        data.inner(),
        &namespaces,
        &inheritable,
        &resolver,
    );
    let mut data = RuntimeValue::new(interpolated);

    let mut errors = 0;

    let compute_errors =
        ac_fields::compute_fields(data.inner_mut(), &computed, &mut namespaces, &resolver);
    for msg in &compute_errors {
        eprintln!("  {msg}");
    }
    errors += compute_errors.len() as i32;

    let merge_errors = ac_fields::merge_fields(data.inner_mut(), &merge);
    for msg in &merge_errors {
        eprintln!("  {msg}");
    }
    errors += merge_errors.len() as i32;

    let schemas: Vec<(String, JsonSchema)> = field_schemas
        .iter()
        .filter(|(name, _)| !merge.iter().any(|m| m == name))
        .map(|(name, raw)| (name.clone(), JsonSchema::new(raw.clone())))
        .collect();

    println!("fields:");
    for (name, schema) in &schemas {
        let type_name = schema.type_name().unwrap_or("any");
        println!("  {name}: {type_name}");
    }
    println!();

    for (name, schema) in &schemas {
        let field_value = data.get(name).unwrap_or(&serde_json::Value::Null);
        let field_rv = RuntimeValue::new(field_value.clone());
        match schema.validate(&field_rv) {
            Ok(()) => {
                println!("  {name}: ok");
            }
            Err(e) => {
                eprintln!("  {name}: INVALID — {e}");
                errors += 1;
            }
        }
    }

    let declared: Vec<&str> = schemas.iter().map(|(n, _)| n.as_str()).collect();
    for key in ac_fields::undeclared_fields(&data, &declared) {
        eprintln!("  {key}: UNDECLARED — not in fields:");
        errors += 1;
    }

    for msg in ac_fields::validate_mutable(data.inner(), &immutable) {
        eprintln!("  {msg}");
        errors += 1;
    }

    if errors == 0 {
        println!("\nall fields valid");
        0
    } else {
        eprintln!("\n{errors} validation error(s)");
        1
    }
}
