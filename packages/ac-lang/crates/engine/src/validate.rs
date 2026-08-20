use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_frontend::frontend_for;
use ac_schema::Schema;

pub fn run(input: &str, format: &str, schema_str: &str) -> Result<(), String> {
    let frontend = frontend_for(format)?;
    let (value, source_map) = frontend.parse_with_locations(input)?;

    let vocab = AC_DIALECT.schema_vocabulary();
    let schema = vocab.compile(&serde_json::from_str(schema_str).map_err(|e| format!("failed to parse schema JSON: {e}"))?)
        .map_err(|e| format!("failed to parse schema: {e}"))?;

    schema.validate(&value).map_err(|e| {
        let location = source_map.resolve_path(&e.path);
        if let Some(loc) = location {
            format!("[{loc}] {e}")
        } else {
            e.to_string()
        }
    })
}
