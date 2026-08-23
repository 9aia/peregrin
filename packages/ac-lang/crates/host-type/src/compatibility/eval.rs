//! Evaluate `is_compatible` as an AC-Lang function value.

use std::collections::HashMap;

use ac_fields::{DEFAULT_NAMESPACE, Namespaces};
use ac_vm::Error;
use serde_json::{Map, Value as JsonValue};

use super::{IS_COMPATIBLE_BODY, is_compatible_function_value};
use crate::eval::TypeEval;
use crate::host::TypeHost;

/// Evaluate `is_compatible` using the AC-Lang function value.
///
/// Sets up a `TypeEval` with the `is_compatible` function value in the
/// default namespace, then evaluates the body expression with `actual`
/// and `expected` bound as parameters.
pub fn eval_is_compatible_ac_lang(actual: &JsonValue, expected: &JsonValue) -> Result<bool, Error> {
    let host = TypeHost::new();
    let is_compatible_fn = is_compatible_function_value();

    // Build namespaces: doc.is_compatible = function value
    let mut doc_obj = Map::new();
    doc_obj.insert("is_compatible".to_string(), is_compatible_fn.to_json());
    let mut namespaces: Namespaces = HashMap::new();
    namespaces.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc_obj));

    let eval = TypeEval::with_namespaces(host, namespaces);

    // Build params for eval_body: actual + expected merged into doc
    let mut merged = eval.namespaces().clone();
    let mut doc = merged
        .get(DEFAULT_NAMESPACE)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    doc.insert("actual".to_string(), actual.clone());
    doc.insert("expected".to_string(), expected.clone());
    merged.insert(DEFAULT_NAMESPACE.to_string(), JsonValue::Object(doc));

    let result = eval.eval_lazy(IS_COMPATIBLE_BODY, &merged).ok_or_else(|| {
        Error::PrimitiveError("failed to evaluate is_compatible body".to_string())
    })?;

    result.as_bool().ok_or(Error::TypeMismatch {
        expected: "boolean",
        got: "non-boolean",
    })
}
