use ac_dialect::Dialect;
use ac_runtime::RuntimeValue;

use crate::types::{
    ArgSpec, ComputedField, ExtractedFields,
};

/// Extracts the `$` key from a value, returning
/// `(field_schemas, data_without_fields)`.
///
/// Each entry in `field_schemas` is `(name, raw_schema_value)`.
/// The caller is responsible for constructing `Schema<V>`
/// implementations from the raw values.
///
/// Key names are read from the dialect — a custom dialect may use
/// different key names for `$`, `expr`, etc.
///
/// Returns an error if a declared field name is reserved.
pub fn extract_fields(
    value: RuntimeValue,
    dialect: &dyn Dialect,
) -> Result<ExtractedFields, String> {
    let mut schemas = Vec::new();

    let fields_key = dialect.document_vocabulary().fields_key();
    let reserved = dialect.document_vocabulary().top_level_reserved_keys();

    // Borrow the value to extract schemas, then move it into data.
    if let Some(obj) = value.as_object() {
        if let Some(fields) = obj.get(fields_key).and_then(|v| v.as_object()) {
            for (name, schema_val) in fields {
                if reserved.contains(&name.as_str()) {
                    return Err(format!(
                        "'{name}' is a reserved field name and cannot be declared in {fields_key}:"
                    ));
                }
                schemas.push((name.clone(), schema_val.clone()));
            }
        }
    }

    // Move the value into data (no clone), then remove the `$` key
    // so data contains only the document's own values.
    let mut data = value;
    if let Some(data_obj) = data.as_object_mut() {
        data_obj.remove(fields_key);
    }

    // Single-pass extraction of inheritable, immutable, computed, and merge
    // field metadata from schemas.
    let mut inheritable = Vec::new();
    let mut immutable = Vec::new();
    let mut computed = Vec::new();
    let mut merge = Vec::new();

    let inheritable_key = dialect.document_vocabulary().inheritable_key();
    let mutable_key = dialect.document_vocabulary().mutable_key();
    let expr_key = dialect.document_vocabulary().expr_key();
    let arg_key = dialect.document_vocabulary().arg_key();
    let merge_key = dialect.document_vocabulary().merge_key();

    for (name, raw) in &schemas {
        if raw.get(inheritable_key).and_then(|v| v.as_bool()) == Some(true) {
            inheritable.push(name.clone());
        }
        if raw.get(mutable_key).and_then(|v| v.as_bool()) == Some(false) {
            immutable.push(name.clone());
        }
        if let Some(expr_str) = raw.get(expr_key).and_then(|v| v.as_str()) {
            let arg = raw.get(arg_key).and_then(|v| {
                if v.is_boolean() && v.as_bool() == Some(true) {
                    Some(ArgSpec::Untyped)
                } else if v.is_object() {
                    Some(ArgSpec::Typed(v.clone()))
                } else {
                    None
                }
            });
            computed.push(ComputedField {
                name: name.clone(),
                expr: expr_str.to_string(),
                arg,
            });
        }
        if raw.get(merge_key).and_then(|v| v.as_bool()) == Some(true) {
            merge.push(name.clone());
        }
    }

    Ok(ExtractedFields { schemas, data, inheritable, immutable, computed, merge })
}

/// Returns the names of fields present in `data` but not declared in
/// `declared_names`.
pub fn undeclared_fields<'a>(
    data: &'a RuntimeValue,
    declared: &[&str],
) -> Vec<&'a str> {
    let declared_set: std::collections::HashSet<&str> = declared.iter().copied().collect();
    data.as_object()
        .map(|obj| {
            obj.keys()
                .filter(|k| !declared_set.contains(k.as_str()))
                .map(|k| k.as_str())
                .collect()
        })
        .unwrap_or_default()
}
