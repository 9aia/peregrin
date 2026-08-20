use ac_runtime::RuntimeValue;
use ac_schema::Schema;

/// Validates function call arguments against declared param schemas.
///
/// Returns `Ok(())` if all args validate, or an error message.
/// The caller provides a `Schema<V>` implementation for each param.
pub fn validate_args<S: Schema<RuntimeValue>>(
    args: &[RuntimeValue],
    param_schemas: &[S],
) -> Result<(), String> {
    if args.len() != param_schemas.len() {
        return Err(format!(
            "argument count mismatch: expected {}, got {}",
            param_schemas.len(),
            args.len()
        ));
    }
    for (i, (arg, schema)) in args.iter().zip(param_schemas.iter()).enumerate() {
        schema.validate(arg).map_err(|e| format!("arg {i}: {e}"))?;
    }
    Ok(())
}

/// Validates a function return value against a declared return schema.
///
/// Returns `Ok(())` if the value validates, or an error message.
pub fn validate_return<S: Schema<RuntimeValue>>(
    value: &RuntimeValue,
    return_schema: &S,
) -> Result<(), String> {
    return_schema.validate(value).map_err(|e| format!("return: {e}"))
}
