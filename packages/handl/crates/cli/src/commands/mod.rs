pub mod compile;
pub mod eval;
pub mod typecheck;

/// Reject input formats that are not `handl` or `toml`.
pub fn ensure_input_format(format: &str) -> Result<(), String> {
    match format {
        "handl" | "toml" => Ok(()),
        other => Err(format!(
            "unsupported input format: {other} (supported: handl, toml)"
        )),
    }
}

/// Reject output formats other than `toml`.
pub fn ensure_output_format(format: &str) -> Result<(), String> {
    match format {
        "toml" => Ok(()),
        other => Err(format!(
            "unsupported output format: {other} (supported: toml)"
        )),
    }
}
