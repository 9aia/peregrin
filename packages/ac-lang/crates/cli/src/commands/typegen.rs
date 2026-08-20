use std::path::Path;

use ac_fields::extract_fields;
use ac_typegen::{TypegenBackend, TypegenContext};
use ac_typegen_json::{JsonSchemaBackend, inject_schema_comment, write_schema_file};

use crate::cli::TypegenArgs;
use crate::frontend;
use crate::io;

pub fn run(args: TypegenArgs) -> i32 {
    let input = match io::read_input(&Some(args.input.clone())) {
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

    let extracted = match extract_fields(value) {
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

    let data = extracted.data.inner().clone();
    let base_path = args.input.parent();
    let ctx = TypegenContext::from_extracted(&extracted, &data, base_path);

    match args.output_format.as_str() {
        "json-schema" => {
            let backend = JsonSchemaBackend;
            let schema = backend.generate(&ctx);

            if args.stdout {
                println!("{}", serde_json::to_string_pretty(&schema).unwrap());
                return 0;
            }

            let defs_dir = &args.output_dir;
            let filename = args.input.file_stem().and_then(|s| s.to_str()).unwrap_or("schema");
            let schema_rel = format!("{filename}.{}", backend.extension());

            let schema_path = defs_dir.join(&schema_rel);
            if let Err(e) = write_schema_file(&schema, &schema_path) {
                eprintln!("error: {e}");
                return 1;
            }

            let rel_for_hint = Path::new(".ac")
                .join("generated")
                .join("schemas")
                .join(&schema_rel);
            if let Err(e) = inject_schema_comment(&args.input, &rel_for_hint.to_string_lossy()) {
                eprintln!("error: {e}");
                return 1;
            }

            println!("schema written to {}", schema_path.display());
            0
        }
        other => {
            eprintln!("error: unknown output format: {other}");
            eprintln!("  available: json-schema");
            1
        }
    }
}
