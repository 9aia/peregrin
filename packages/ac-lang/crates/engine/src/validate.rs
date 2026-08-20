use ac_frontend::frontend_for;
use ac_schema::Schema;
use ac_schema_json::JsonSchema;

pub fn run(input: &str, format: &str, schema_str: &str) -> Result<(), String> {
    let frontend = frontend_for(format)?;
    let value = frontend.parse(input)?;

    let schema = JsonSchema::parse(schema_str).map_err(|e| format!("failed to parse schema: {e}"))?;

    schema.validate(&value).map_err(|e| {
        let map = ac_source::SourceMap::new(input);
        let location = map.resolve_path(&e.path);
        if let Some(loc) = location {
            format!("[{loc}] {e}")
        } else {
            e.to_string()
        }
    })
}
