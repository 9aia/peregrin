use serde_json::Value as JsonValue;

/// Spreads `merge: true` field values into the parent scope, then removes
/// the field itself.
///
/// After computation, fields declared with `merge: true` have their values
/// (which must be objects) flattened into the parent data object. The field
/// itself is removed after the spread.
///
/// Conflict resolution: parent wins — keys already present in `data` are
/// not overwritten by merged values. Multiple `merge: true` fields merge in
/// declaration order; earlier merges fill keys that later merges cannot
/// overwrite.
///
/// Top-level spread only — no deep merge. If a merge field's value is not
/// an object, an error is returned for that field.
///
/// Returns a list of error messages for any merge fields that could not
/// be spread.
pub fn merge_fields(
    data: &mut JsonValue,
    merge: &[String],
) -> Vec<String> {
    if merge.is_empty() {
        return Vec::new();
    }

    let mut errors = Vec::new();

    for field_name in merge {
        let obj = match data.as_object_mut() {
            Some(o) => o,
            None => {
                errors.push(format!(
                    "merge: data is not an object, cannot merge '{field_name}'"
                ));
                continue;
            }
        };

        // Remove the merge field's value from the data object.
        let field_value = match obj.remove(field_name) {
            Some(v) => v,
            None => continue, // Field not present in data — nothing to merge.
        };

        // The value must be an object to spread.
        let spread_map = match field_value.as_object() {
            Some(m) => m.clone(),
            None => {
                // Put the value back since we can't merge it.
                obj.insert(field_name.clone(), field_value);
                errors.push(format!(
                    "merge: field '{field_name}' value is not an object — cannot spread"
                ));
                continue;
            }
        };

        // Spread keys into the parent. Parent wins — don't overwrite existing keys.
        for (key, val) in spread_map {
            if !obj.contains_key(&key) {
                obj.insert(key, val);
            }
        }
    }

    errors
}
