//! Convert/import targets — the stores the node's
//! convert/import routes accept (`ImportTarget` in
//! `peregrin-http`'s `routes/sessions.rs`).

/// Convert/import stores the node accepts (`ImportTarget` in
/// `routes/sessions.rs` — `{agent: "cline"|"devin"}`). `(id, label)`.
pub const CONVERT_TARGETS: [(&str, &str); 2] = [("cline", "Cline"), ("devin", "Devin")];

/// Whether `target` is a store the node's convert/import routes take.
pub fn is_convert_target(target: &str) -> bool {
    CONVERT_TARGETS.iter().any(|(id, _)| *id == target)
}

/// Convert targets minus the store the session already lives in —
/// converting devin→devin is a no-op the UI shouldn't offer.
pub fn convert_targets(current_agent: &str) -> Vec<(&'static str, &'static str)> {
    CONVERT_TARGETS
        .into_iter()
        .filter(|(id, _)| *id != current_agent)
        .collect()
}
