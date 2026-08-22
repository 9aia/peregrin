use ac_compiler_core::{Compilation, Config, PassError};
use serde_json::Value as JsonValue;

/// Run the typecheck logic, populating `compilation.type_errors`.
pub fn run_typecheck(compilation: &mut Compilation, config: &Config) -> Result<(), PassError> {
    let extracted = compilation
        .extracted
        .as_ref()
        .ok_or_else(|| PassError::new("typecheck", "no extracted fields — run ExtractPass first"))?;

    let interpreter = config.interpreter.as_ref();
    let data = extracted.data.inner().clone();

    let mut env = ac_typecheck_core::TypeEnv::new();
    for (name, raw) in &extracted.schemas {
        if !extracted.merge.contains(name) {
            env.insert(name.clone(), interpreter.interpret(raw));
        }
    }

    let fn_sigs: Vec<ac_typecheck_infer::RawFnSig> = extracted
        .schemas
        .iter()
        .filter_map(|(name, raw)| {
            let type_name = raw.get("type").and_then(|v| v.as_str())?;
            if type_name != "function" {
                return None;
            }

            let type_params = raw
                .get("typeParams")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let params = raw
                .get("params")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let pname = p.get("name")?.as_str()?.to_string();
                            let pschema = p.get("schema").cloned().unwrap_or(JsonValue::Null);
                            Some((pname, pschema))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let return_schema = raw.get("return").cloned();

            Some((name.clone(), type_params, params, return_schema))
        })
        .collect();

    // Merge host function signatures from Config extension (if provided).
    let mut all_sigs = fn_sigs;
    if let Some(host_sigs) = config.extension::<ac_typecheck_infer::HostFunctionSigs>() {
        all_sigs.extend(host_sigs.0.iter().cloned());
    }

    let function_table = ac_typecheck_infer::FunctionTable::from_signatures(&all_sigs, interpreter);

    let inheritable_schemas: Vec<(String, serde_json::Value)> = extracted
        .schemas
        .iter()
        .filter(|(name, _)| extracted.inheritable.contains(name))
        .cloned()
        .collect();

    let base = compilation.source.base_path();

    let (mut local_errors, data_errors) = rayon::join(
        || {
            let (mut expr_errors, (arg_errors, merge_errors)) = rayon::join(
                || {
                    ac_typecheck_key_expr::check(
                        &extracted.schemas,
                        &env,
                        &function_table,
                        interpreter,
                    )
                },
                || {
                    rayon::join(
                        || ac_typecheck_key_arg::check(&extracted.schemas, interpreter),
                        || ac_typecheck_key_merge::check(&extracted.schemas, interpreter, &env),
                    )
                },
            );
            expr_errors.extend(arg_errors);
            expr_errors.extend(merge_errors);
            expr_errors
        },
        || {
            let (mut inherit_errors, (mutable_errors, import_errors)) = rayon::join(
                || ac_typecheck_key_inherit::check(&inheritable_schemas, &data, interpreter),
                || {
                    rayon::join(
                        || ac_typecheck_key_mutable::check(&extracted.immutable, &data),
                        || {
                            ac_typecheck_import::check_imports(
                                &extracted.schemas,
                                &extracted.merge,
                                &data,
                                config.resolver.as_ref(),
                                interpreter,
                                &env,
                                base.as_deref(),
                            )
                        },
                    )
                },
            );
            inherit_errors.extend(mutable_errors);
            inherit_errors.extend(import_errors);
            inherit_errors
        },
    );
    local_errors.extend(data_errors);

    compilation.type_errors = local_errors;
    Ok(())
}
