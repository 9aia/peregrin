use crate::expression::eval_expression;
use crate::types::{ArgSpec, ComputedField, FunctionResolver, Namespaces};
use serde_json::Value as JsonValue;

/// Evaluates computed field expressions and inserts results into `data`.
///
/// Fields declared with an `expr:` key are computed after interpolation.
/// The expression uses the same syntax as `${...}` content: function calls,
/// path lookups (via namespaces), and literals. Computed fields can reference
/// other fields via the `doc` namespace and can depend on other computed
/// fields — multiple passes resolve dependencies in order.
///
/// The `doc` namespace in `namespaces` is updated with `data` before each
/// pass so expressions see current field values.
///
/// Returns a list of error messages for any computed fields that could not
/// be evaluated after all passes.
pub fn compute_fields(
    data: &mut JsonValue,
    computed: &[ComputedField],
    namespaces: &mut Namespaces,
    resolver: &dyn FunctionResolver,
) -> Vec<String> {
    if computed.is_empty() {
        return Vec::new();
    }

    let mut errors = Vec::new();
    let mut remaining: Vec<ComputedField> = computed.to_vec();

    // Multiple passes to resolve dependencies between computed fields.
    // At most `computed.len()` passes — each pass resolves at least one
    // field, or we stop and report the rest as errors.
    for _pass in 0..computed.len() {
        if remaining.is_empty() {
            break;
        }

        // Sync doc namespace with current data state
        namespaces.insert("doc".to_string(), data.clone());

        let mut still_remaining = Vec::new();
        for field in &remaining {
            // If the field has an arg, get its current data value
            // and insert it as the `arg` namespace.
            if let Some(arg_spec) = &field.arg
                && let Some(obj) = data.as_object()
                && let Some(arg_val) = obj.get(&field.name)
            {
                // Validate arg type if declared
                if let ArgSpec::Typed(schema) = arg_spec
                    && let Err(e) = check_arg_type(arg_val, schema)
                {
                    errors.push(format!(
                        "could not compute field '{}': {e}", field.name
                    ));
                    continue;
                }
                namespaces.insert("arg".to_string(), arg_val.clone());
            }

            if let Some(val) = eval_expression(&field.expr, namespaces, resolver) {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert(field.name.clone(), val);
                }
            } else {
                still_remaining.push(field.clone());
            }

            // Clean up arg namespace
            if field.arg.is_some() {
                namespaces.remove("arg");
            }
        }

        if still_remaining.len() == remaining.len() {
            // No progress — remaining fields can't be resolved
            for field in &still_remaining {
                errors.push(format!(
                    "could not compute field '{}': failed to evaluate '{}'",
                    field.name, field.expr
                ));
            }
            break;
        }

        remaining = still_remaining;
    }

    errors
}

/// Minimal type check for arg validation.
///
/// Checks the `type` field of a raw schema value against the actual value type.
/// This is a minimal check — full schema validation should use a `Schema<V>` implementation.
fn check_arg_type(value: &JsonValue, schema: &JsonValue) -> Result<(), String> {
    if let Some(type_name) = schema.get("type").and_then(|v| v.as_str()) {
        let actual = match value {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        };
        if type_name == "integer" {
            if let JsonValue::Number(n) = value
                && n.as_f64().is_some_and(|f| f.fract() == 0.0)
            {
                return Ok(());
            }
            return Err(format!(
                "arg: expected type integer, got {actual}"
            ));
        }
        if actual != type_name {
            return Err(format!(
                "arg: expected type {type_name}, got {actual}"
            ));
        }
    }
    Ok(())
}
