use serde_json::Value as JsonValue;

/// Validates that fields declared `mutable: false` are not overridden in
/// nested scopes.
///
/// Walks the data tree. For each immutable field, compares every nested
/// occurrence against the root value. Returns a list of error messages
/// for any mismatches.
pub fn validate_mutable(
    data: &JsonValue,
    immutable: &[String],
) -> Vec<String> {
    if immutable.is_empty() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    if let Some(obj) = data.as_object() {
        for field in immutable {
            if let Some(root_val) = obj.get(field) {
                check_mutable_in_children(data, field, root_val, &mut errors, "");
            }
        }
    }
    errors
}

/// Recursively checks that `field` is not overridden in nested objects.
fn check_mutable_in_children(
    value: &JsonValue,
    field: &str,
    root_val: &JsonValue,
    errors: &mut Vec<String>,
    path: &str,
) {
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            let child_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            if key == field && val != root_val {
                errors.push(format!(
                    "{child_path}: '{field}' is immutable (mutable: false) \
                     — cannot override value from root scope"
                ));
            }
            if val.is_object() {
                check_mutable_in_children(val, field, root_val, errors, &child_path);
            }
        }
    }
}
