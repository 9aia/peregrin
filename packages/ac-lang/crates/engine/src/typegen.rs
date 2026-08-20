use std::path::{Path, PathBuf};

use ac_typegen_backend_json_schema::{inject_schema_comment, write_schema_file};

use crate::Source;

/// Input parameters for a typegen run.
pub struct TypegenInput<'a> {
    pub input: &'a str,
    pub format: &'a str,
    pub input_path: &'a Path,
    pub output_format: &'a str,
    pub output_dir: &'a Path,
    pub stdout: bool,
}

/// Result of a typegen run.
pub enum TypegenResult {
    /// Schema was printed to stdout (the serialized schema).
    Stdout(String),
    /// Schema was written to a file (the file path).
    FileWritten(PathBuf),
}

pub fn run(args: &TypegenInput) -> Result<TypegenResult, String> {
    let source = Source::new(args.input, args.format)
        .with_path(args.input_path);

    let pipeline = crate::typegen_pipeline(crate::default_cli_config());
    let compilation = pipeline.run(source)?;

    let output = compilation
        .output
        .ok_or("typegen: no output generated")?;

    match args.output_format {
        "json-schema" => {
            if args.stdout {
                let json = serde_json::to_string_pretty(&output)
                    .map_err(|e| e.to_string())?;
                return Ok(TypegenResult::Stdout(json));
            }

            let filename = args.input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("schema");
            let schema_rel = format!("{filename}.json");

            let schema_path = args.output_dir.join(&schema_rel);
            write_schema_file(&output, &schema_path)?;

            let rel_for_hint = Path::new(".ac")
                .join("generated")
                .join("schemas")
                .join(&schema_rel);
            inject_schema_comment(args.input_path, &rel_for_hint.to_string_lossy())?;

            Ok(TypegenResult::FileWritten(schema_path))
        }
        other => Err(format!("unknown output format: {other}\n  available: json-schema")),
    }
}
