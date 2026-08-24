pub mod compile;
pub mod eval;
pub mod fields;
pub mod parse;
pub mod typecheck;
pub mod typegen;
pub mod validate;

/// Reject input formats that are not `ac` or `toml`.
pub fn ensure_input_format(format: &str) -> Result<(), String> {
    match format {
        "ac" | "toml" => Ok(()),
        other => Err(format!("unsupported input format: {other} (supported: ac, toml)")),
    }
}

/// Reject output formats other than `toml`.
pub fn ensure_output_format(format: &str) -> Result<(), String> {
    match format {
        "toml" => Ok(()),
        other => Err(format!("unsupported output format: {other} (supported: toml)")),
    }
}
