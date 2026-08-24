use ac_compiler_core::{Compilation, Config, PassError};
use ac_typecheck_core::{Type, TypeParam, Variance};
use serde_json::Value as JsonValue;

/// Parse a typeParams schema entry into a TypeParam.
///
/// Supports two forms:
/// - String: `"T"` → `TypeParam::new("T")`
/// - Object: `{ "name": "T", "bounds": [{ "type": "trait", "name": "Numeric" }], "variance": "covariant" }`
fn parse_type_param(v: &JsonValue) -> Option<TypeParam> {
    if let Some(s) = v.as_str() {
        return Some(TypeParam::new(s));
    }
    let obj = v.as_object()?;
    let name = obj.get("name")?.as_str()?.to_string();
    let bounds = obj
        .get("bounds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|b| {
                    let btype = b.get("type")?.as_str()?;
                    match btype {
        "trait" => b.get("name")?.as_str().map(|n| Type::Trait(n.to_string())),
        _ => None}
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let variance = obj
        .get("variance")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "covariant" | "out" => Some(Variance::Covariant),
            "contravariant" | "in" => Some(Variance::Contravariant),
            "invariant" => Some(Variance::Invariant),
            "bivariant" => Some(Variance::Bivariant),
            _ => None})
        .unwrap_or(Variance::Invariant);
    let tp = if bounds.is_empty() {
        TypeParam::new(name)
    } else {
        TypeParam::with_bounds(name, bounds)
    };
    Some(tp.with_variance(variance))
}

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
                        .filter_map(parse_type_param)
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
            let (mut expr_errors, ((arg_errors, merge_errors), termination_errors)) = rayon::join(
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
                        || {
                            rayon::join(
                                || ac_typecheck_key_arg::check(&extracted.schemas, interpreter),
                                || ac_typecheck_key_merge::check(&extracted.schemas, interpreter, &env),
                            )
                        },
                        || {
                            let bodies = ac_typecheck_key_termination::extract_function_bodies(&data);
                            ac_typecheck_key_termination::check(&bodies, &ac_dialect_canonical::AC_DIALECT)
                        },
                    )
                },
            );
            expr_errors.extend(arg_errors);
            expr_errors.extend(merge_errors);
            expr_errors.extend(termination_errors);
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

    // Store the type environment and function table for downstream
    // consumers (e.g. generic type arg resolution during evaluation).
    compilation.store(env);
    compilation.store(function_table);

    Ok(())
}