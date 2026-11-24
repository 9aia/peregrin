//! `params.schema.json` — bundle `config:` validation (ADR-0081,
//! I-022).
//!
//! A `peregrin-bundle` ships a JSON Schema (draft 2020-12) next to its
//! `main.yaml` declaring the operator-facing `config:` keys it
//! accepts. Two boundaries enforce it:
//!
//! - the **loader** — [`crate::Loader`] impls check the merged
//!   `config:` mapping when a `params.schema.json` sits beside the
//!   loaded file, so a typo'd key fails the load instead of silently
//!   resolving `${config.*}` to an empty string;
//! - **`peregrin provision {install,deploy}`** — the provider driver
//!   validates the bundle's own `config:` defaults *and* the
//!   `config:` contributed by `TAMIAS_DEPLOY_OVERRIDES` fragments
//!   before writing the node config, so bad operator params never
//!   reach `~/.config/peregrin/serve/`.
//!
//! The file is opt-in: absent schema → no validation (plain campaigns
//! carry arbitrary `config:` keys).

use serde_json::{Map, Value};

/// The schema file name a bundle drops beside `main.yaml`.
pub const PARAMS_SCHEMA_FILE: &str = "params.schema.json";

/// Validate a `config:` mapping against a parsed `params.schema.json`.
///
/// `Ok` lists every violation (empty = valid); `Err` means the schema
/// document itself failed to compile — a broken schema is a bundle
/// bug and must surface, never be skipped.
///
/// # Errors
/// Returns `Err` when `schema` is not a usable JSON Schema.
pub fn params_violations(
    schema: &Value,
    config: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    // scarab-validate's `Violation` Display is `path: message` (bare
    // message at the root) — the same strings this function used to
    // format by hand.
    let compiled = scarab_validate::Schema::compile(schema)?;
    let instance = Value::Object(config.clone());
    Ok(compiled
        .validate(&instance)
        .iter()
        .map(ToString::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use super::*;

    fn schema() -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": ["token_env"],
            "properties": {
                "token_env": {"type": "string", "pattern": "^[A-Z_]+$"},
                "timeout": {"type": "integer", "minimum": 1, "maximum": 600},
                "repos": {"type": "array", "items": {"type": "string"}}
            }
        })
    }

    fn cfg_map(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn valid_config_has_no_violations() {
        let config = cfg_map(&[
            ("token_env", json!("BUPHA_TOKEN")),
            ("timeout", json!(600)),
            ("repos", json!(["9aia/peregrin"])),
        ]);
        assert_eq!(
            params_violations(&schema(), &config).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn typoed_key_rejects_under_additional_properties_false() {
        let config = cfg_map(&[("token_env", json!("BUPHA_TOKEN")), ("timout", json!(600))]);
        let errors = params_violations(&schema(), &config).unwrap();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("timout"), "{errors:?}");
    }

    #[test]
    fn missing_required_rejects() {
        let config = cfg_map(&[("timeout", json!(60))]);
        let errors = params_violations(&schema(), &config).unwrap();
        assert!(errors.iter().any(|e| e.contains("token_env")), "{errors:?}");
    }

    #[test]
    fn wrong_type_and_bounds_reject() {
        let config = cfg_map(&[
            ("token_env", json!("BUPHA_TOKEN")),
            ("timeout", json!("600")),
        ]);
        let errors = params_violations(&schema(), &config).unwrap();
        assert!(errors.iter().any(|e| e.contains("timeout")), "{errors:?}");

        let config = cfg_map(&[("token_env", json!("BUPHA_TOKEN")), ("timeout", json!(0))]);
        let errors = params_violations(&schema(), &config).unwrap();
        assert!(errors.iter().any(|e| e.contains("minimum")), "{errors:?}");

        let config = cfg_map(&[("token_env", json!("lowercase"))]);
        let errors = params_violations(&schema(), &config).unwrap();
        assert!(
            errors.iter().any(|e| e.contains("does not match")),
            "{errors:?}"
        );
    }

    #[test]
    fn unusable_schema_is_an_error_not_a_pass() {
        let config = cfg_map(&[("token_env", json!("BUPHA_TOKEN"))]);
        let bad = json!({"type": "not-a-type"});
        assert!(params_violations(&bad, &config).is_err());
    }
}
